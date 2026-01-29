//! # Cache Management
//!
//! This module provides high-level cache management operations for AArch64 processors.
//! It includes functions for managing data cache (dcache) and instruction cache (icache),
//! as well as utilities for performing cache operations on specific memory ranges, values,
//! and entire cache levels.
//!
//! ## Overview
//!
//! The cache management functions support three types of operations:
//! - **Clean**: Write back dirty cache lines to memory
//! - **Invalidate**: Mark cache lines as invalid (without writing back)
//! - **Clean and Invalidate**: Write back dirty lines and then invalidate them
//!
//! ## Functions
//!
//! - [`icache_flush_all`]: Flushes the entire instruction cache
//! - [`dcache_range`]: Performs cache operations on a memory range
//! - [`dcache_value`]: Performs cache operations on a specific value
//! - [`dcache_all`]: Performs cache operations on all data cache levels
//! - [`cache_line_size`]: Returns the system cache line size
//!
//! ## Notes
//!
//! Cache operations typically require appropriate memory barriers to ensure visibility
//! across cores. This module automatically inserts necessary barriers (DSB and ISB)
//! after cache operations as required by the architecture.

use core::arch::asm;

use aarch64_cpu::{
    asm::barrier::{NSH, SY, dsb, isb},
    registers::*,
};

use crate::asm::cache::{CISW, CIVAC, CSW, CVAC, IALLU, ISW, IVAC, dc, ic};

/// Flushes the entire instruction cache.
///
/// This function invalidates all entries in the instruction cache to ensure that
/// any recent code modifications become visible to the processor. It performs the
/// following operations:
///
/// 1. Invalidate all instruction caches at all levels using `IC IALLU`
/// 2. Execute a data synchronization barrier (DSB) to ensure completion
/// 3. Execute an instruction synchronization barrier (ISB) to ensure context synchronization
///
/// This is typically called after modifying code that may be cached, such as when
/// applying patches, loading modules, or generating code at runtime.
pub fn icache_flush_all() {
    ic(IALLU);
    dsb(NSH);
    isb(SY);
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum CacheOp {
    /// Write back to memory
    Clean,
    /// Invalidate cache
    Invalidate,
    /// Clean and invalidate
    CleanAndInvalidate,
}

/// Returns the cache line size in bytes.
///
/// This function reads the CTR_EL0 (Cache Type Register) to determine the
/// minimum data cache line size. The line size is calculated from the DminLine
/// field which contains log2 of the number of words in the smallest cache line.
///
/// # Returns
///
/// The cache line size in bytes (typically 32, 64, or 128 bytes).
#[inline(always)]
pub fn cache_line_size() -> usize {
    unsafe {
        let mut ctr_el0: u64;
        asm!("mrs {}, ctr_el0", out(reg) ctr_el0);
        // CTR_EL0.DminLine (bits 19:16) - log2 of the number of words in the smallest cache line
        let log2_cache_line_size = ((ctr_el0 >> 16) & 0xF) as usize;
        // Calculate the cache line size: 4 * (2^log2_cache_line_size) bytes
        4 << log2_cache_line_size
    }
}

/// Represents the type of cache operation to perform.
///
/// This enum specifies whether a cache operation should clean, invalidate,
/// or perform both clean and invalidate operations on cache lines.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum CacheOp {
    /// Write back to memory.
    ///
    /// This operation writes dirty cache lines back to main memory without invalidating them.
    /// The cache lines remain valid and can be accessed without going to memory.
    Clean,
    /// Invalidate cache.
    ///
    /// This operation marks cache lines as invalid. If the cache line is dirty,
    /// the data is lost. Use this with caution as it can lead to data loss.
    Invalidate,
    /// Clean and invalidate.
    ///
    /// This operation first writes dirty cache lines back to memory and then
    /// invalidates them. This ensures data consistency while forcing subsequent
    /// accesses to fetch fresh data from memory.
    CleanAndInvalidate,
}

/// Performs a cache operation on a single cache line.
#[inline]
fn _dcache_line(op: CacheOp, addr: usize) {
    let addr = addr as u64;
    match op {
        CacheOp::Clean => dc(CVAC, addr),
        CacheOp::Invalidate => dc(IVAC, addr),
        CacheOp::CleanAndInvalidate => dc(CIVAC, addr),
    }
}

/// Performs a cache operation on a range of memory.
///
/// This function iterates over cache lines in the specified memory range and
/// performs the specified cache operation on each line. The operation is
/// performed on aligned cache line addresses to ensure correct behavior.
///
/// # Arguments
///
/// * `op` - The type of cache operation to perform
/// * `addr` - The starting address of the memory range
/// * `size` - The size of the memory range in bytes
///
/// # Notes
///
/// - The function aligns the start address to the cache line boundary
/// - Data synchronization and instruction synchronization barriers are
///   automatically inserted after the operation completes
#[inline]
pub fn dcache_range(op: CacheOp, addr: usize, size: usize) {
    let start = addr;
    let end = start + size;
    let cache_line_size = cache_line_size();

    let mut aligned_addr = addr & !(cache_line_size - 1);

    while aligned_addr < end {
        _dcache_line(op, aligned_addr);
        aligned_addr += cache_line_size;
    }

    dsb(SY);
    isb(SY);
}

/// Performs a cache operation on a value.
///
/// This is a convenience function that operates on a specific value rather than
/// a raw memory range. It automatically calculates the address and size of the
/// value and delegates to [`dcache_range`].
///
/// # Type Parameters
///
/// * `T` - The type of the value
///
/// # Arguments
///
/// * `op` - The type of cache operation to perform
/// * `v` - A reference to the value to operate on
pub fn dcache_value<T>(op: CacheOp, v: &T) {
    // Get the pointer to the value
    let ptr = v as *const T as usize;
    // Calculate the size of the value in bytes
    let size = core::mem::size_of_val(v);
    // Perform cache operation on the value
    dcache_range(op, ptr, size);
}

/// Performs a cache operation on a specific cache level using set/way operations.
///
/// This function operates on a specific cache level by iterating through all
/// sets and ways in that cache. It uses the DC (Data Cache) instructions with
/// set/way operands to perform the specified operation on each cache line.
///
/// # Arguments
///
/// * `op` - The type of cache operation to perform
/// * `level` - The cache level (0-7)
///
/// # Panics
///
/// Panics if `level` is greater than 7 (outside the valid ARMv8 range).
///
/// # Technical Details
///
/// The function reads cache parameters from CCSIDR_EL1 to determine:
/// - Line size (in bytes)
/// - Associativity (number of ways)
/// - Number of sets
///
/// It then constructs set/way operands according to the ARM DC instruction format:
/// - Bits [31:4]: Set/way field
///   - Way field: bits[31:32-A] where A = Log2(ASSOCIATIVITY)
///   - Set field: bits[B-1:L] where B = L + S, L = Log2(LINELEN), S = Log2(NSETS)
/// - Bits [3:1]: Cache level (minus 1)
/// - Bit [0]: Reserved (RES0)
///
/// # References
///
/// - [DC CISW Instruction](https://developer.arm.com/documentation/ddi0601/2024-09/AArch64-Instructions/DC-CISW--Data-or-unified-Cache-line-Clean-and-Invalidate-by-Set-Way)
/// - [CTR_EL0 Register](https://developer.arm.com/documentation/ddi0601/2024-09/AArch64-Registers/CTR-EL0--Cache-Type-Register)
/// - [CCSIDR_EL1 Register](https://developer.arm.com/documentation/ddi0601/2024-09/AArch64-Registers/CCSIDR-EL1--Current-Cache-Size-ID-Register)
/// - [U-Boot cache implementation](https://github.com/u-boot/u-boot/blob/master/arch/arm/cpu/armv8/cache.S)
#[inline]
fn dcache_level(op: CacheOp, level: u64) {
    assert!(level < 8, "armv8 level range is 0-7");

    isb(SY);
    CSSELR_EL1.write(CSSELR_EL1::InD::Data + CSSELR_EL1::Level.val(level));
    isb(SY);

    // Read cache parameters from CCSIDR_EL1
    // Note: All values from CCSIDR_EL1 need to be adjusted according to ARM spec:
    // - LineSize: (Log2(bytes in cache line)) - 4
    // - Associativity: (Associativity of cache) - 1
    // - NumSets: (Number of sets in cache) - 1
    let line_size_raw = CCSIDR_EL1.read(CCSIDR_EL1::LineSize) as u32;
    let associativity_raw = CCSIDR_EL1.read(CCSIDR_EL1::AssociativityWithCCIDX) as u32;
    let num_sets_raw = CCSIDR_EL1.read(CCSIDR_EL1::NumSetsWithCCIDX) as u32;

    // Convert raw values to actual values
    let line_size_log2_bytes = line_size_raw + 4; // Actual log2 of line size in bytes
    let associativity = associativity_raw + 1; // Actual associativity
    let num_sets = num_sets_raw + 1; // Actual number of sets

    // Calculate bit positions for set/way encoding according to ARM spec:
    // L = Log2(LINELEN) where LINELEN is line length in bytes
    // S = Log2(NSETS)
    // A = Log2(ASSOCIATIVITY)
    // Way field: bits[31:32-A]
    // Set field: bits[B-1:L] where B = L + S

    let l = line_size_log2_bytes; // Log2 of line length in bytes

    // Calculate the number of bits needed to represent the way index
    // leading_zeros on (associativity-1) gives us the position of the MSB needed
    let way_shift = associativity_raw.leading_zeros(); // Way field starts at bit (32-A)
    let set_shift = l; // Set field starts at bit L (line size offset)

    // Loop over all sets and ways (0-based indexing for hardware)
    for set in 0..num_sets {
        for way in 0..associativity {
            // Construct the set/way value according to ARM DC instruction format:
            // Way field: bits[31:32-A] - way value shifted to proper bit position
            // Set field: bits[B-1:L] - set value shifted to proper bit position
            //
            // Example: If associativity=4, way indices are 0,1,2,3
            // We need A=2 bits (Log2(4)=2), so way field is at bits[31:30]
            // way_shift = 32 - 2 = 30, so way values are shifted left by 30 bits
            let set_way = (way << way_shift) | (set << set_shift);

            // Complete operand: set_way in bits [31:4], level in bits [3:1], bit [0] is RES0
            let cisw = (set_way as u64) | (level << 1);
            match op {
                CacheOp::Invalidate => dc(ISW, cisw),
                CacheOp::Clean => dc(CSW, cisw),
                CacheOp::CleanAndInvalidate => dc(CISW, cisw),
            }
        }
    }
}

/// Performs a cache operation on all data caches.
///
/// This function iterates through all cache levels (0-7) as defined in CLIDR_EL1
/// and performs the specified cache operation on each data cache. It automatically
/// detects the cache type at each level and only processes relevant caches:
///
/// - Data cache only (0b010)
/// - Unified cache (0b100)
/// - Separate instruction and data caches (0b100)
///
/// Instruction-only caches (0b001) and reserved values are skipped.
///
/// # Arguments
///
/// * `op` - The type of cache operation to perform
pub fn dcache_all(op: CacheOp) {
    let clidr = CLIDR_EL1.get();

    for level in 0..8 {
        let ty = (clidr >> (level * 3)) & 0b111;

        // Cache type values:
        // 0b000 = No cache
        // 0b001 = Instruction cache only
        // 0b010 = Data cache only
        // 0b011 = Separate instruction and data caches
        // 0b100 = Unified cache
        // Only process data caches (0b010) and unified caches (0b100)
        // or separate I+D caches (0b011) - for 0b011, we process the data cache
        match ty {
            0b000 => return,   // No cache at this level, we're done
            0b001 => continue, // Instruction cache only, skip
            0b010..=0b100 => {
                // Data cache (0b010), separate I+D caches (0b011), or unified cache (0b100) - process it
                dcache_level(op, level);
            }
            _ => continue, // Reserved values, skip
        }
    }
    dsb(SY);
    isb(SY);
}
