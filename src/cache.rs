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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_op_clone_copy() {
        // Test that CacheOp implements Clone and Copy correctly
        let op = CacheOp::Clean;
        let op_copy = op;
        let op_clone = op.clone();

        assert_eq!(op, op_copy);
        assert_eq!(op, op_clone);
    }

    #[test]
    fn test_cache_op_debug() {
        // Test that CacheOp implements Debug
        let clean = CacheOp::Clean;
        let invalidate = CacheOp::Invalidate;
        let clean_inv = CacheOp::CleanAndInvalidate;

        assert!(format!("{:?}", clean).contains("Clean"));
        assert!(format!("{:?}", invalidate).contains("Invalidate"));
        assert!(format!("{:?}", clean_inv).contains("CleanAndInvalidate"));
    }

    #[test]
    fn test_cache_op_partial_eq() {
        // Test that CacheOp implements PartialEq correctly
        assert_eq!(CacheOp::Clean, CacheOp::Clean);
        assert_eq!(CacheOp::Invalidate, CacheOp::Invalidate);
        assert_eq!(CacheOp::CleanAndInvalidate, CacheOp::CleanAndInvalidate);

        assert_ne!(CacheOp::Clean, CacheOp::Invalidate);
        assert_ne!(CacheOp::Clean, CacheOp::CleanAndInvalidate);
        assert_ne!(CacheOp::Invalidate, CacheOp::CleanAndInvalidate);
    }

    #[test]
    fn test_cache_op_size() {
        // Test that CacheOp is a simple enum with minimal size
        assert_eq!(core::mem::size_of::<CacheOp>(), 1);
    }

    #[test]
    fn test_tte_invalid_default() {
        // Test that invalid TTE has correct default state
        use crate::structures::tte::{Granule4KB, OA48, TTE64};

        let tte = TTE64::<Granule4KB, OA48>::invalid();
        assert!(!tte.is_valid());
        assert!(!tte.is_table());
        assert!(!tte.is_block());
        assert_eq!(tte.get(), 0);
    }

    #[test]
    fn test_granule_size_properties() {
        // Test granule size properties
        use crate::structures::tte::{Granule, Granule16KB, Granule4KB, Granule64KB};

        // Test 4KB granule
        assert_eq!(Granule4KB::M, 12);
        assert_eq!(Granule4KB::SIZE, 4096);
        assert_eq!(Granule4KB::MASK, 0xFFF);

        // Test 16KB granule
        assert_eq!(Granule16KB::M, 14);
        assert_eq!(Granule16KB::SIZE, 16384);
        assert_eq!(Granule16KB::MASK, 0x3FFF);

        // Test 64KB granule
        assert_eq!(Granule64KB::M, 16);
        assert_eq!(Granule64KB::SIZE, 65536);
        assert_eq!(Granule64KB::MASK, 0xFFFF);
    }

    #[test]
    fn test_access_permission_from_bits() {
        use crate::structures::tte::AccessPermission;

        // Test conversion from bits to AccessPermission
        assert_eq!(
            AccessPermission::from_bits(0b00),
            Some(AccessPermission::PrivilegedReadWrite)
        );
        assert_eq!(
            AccessPermission::from_bits(0b01),
            Some(AccessPermission::ReadWrite)
        );
        assert_eq!(
            AccessPermission::from_bits(0b10),
            Some(AccessPermission::PrivilegedReadOnly)
        );
        assert_eq!(
            AccessPermission::from_bits(0b11),
            Some(AccessPermission::ReadOnly)
        );
        assert_eq!(AccessPermission::from_bits(0b100), None); // Invalid bits
    }

    #[test]
    fn test_access_permission_as_bits() {
        use crate::structures::tte::AccessPermission;

        assert_eq!(AccessPermission::PrivilegedReadWrite.as_bits(), 0b00);
        assert_eq!(AccessPermission::ReadWrite.as_bits(), 0b01);
        assert_eq!(AccessPermission::PrivilegedReadOnly.as_bits(), 0b10);
        assert_eq!(AccessPermission::ReadOnly.as_bits(), 0b11);
    }

    #[test]
    fn test_access_permission_checks() {
        use crate::structures::tte::AccessPermission;

        // Test allows_unprivileged
        assert!(!AccessPermission::PrivilegedReadWrite.allows_unprivileged());
        assert!(AccessPermission::ReadWrite.allows_unprivileged());
        assert!(!AccessPermission::PrivilegedReadOnly.allows_unprivileged());
        assert!(AccessPermission::ReadOnly.allows_unprivileged());

        // Test allows_privileged_write
        assert!(AccessPermission::PrivilegedReadWrite.allows_privileged_write());
        assert!(AccessPermission::ReadWrite.allows_privileged_write());
        assert!(!AccessPermission::PrivilegedReadOnly.allows_privileged_write());
        assert!(!AccessPermission::ReadOnly.allows_privileged_write());

        // Test allows_unprivileged_write
        assert!(!AccessPermission::PrivilegedReadWrite.allows_unprivileged_write());
        assert!(AccessPermission::ReadWrite.allows_unprivileged_write());
        assert!(!AccessPermission::PrivilegedReadOnly.allows_unprivileged_write());
        assert!(!AccessPermission::ReadOnly.allows_unprivileged_write());
    }

    #[test]
    fn test_access_permission_partial_eq() {
        use crate::structures::tte::AccessPermission;

        assert_eq!(AccessPermission::ReadWrite, AccessPermission::ReadWrite);
        assert_ne!(AccessPermission::ReadWrite, AccessPermission::ReadOnly);
    }

    #[test]
    fn test_oa_bits() {
        use crate::structures::tte::{OA, OA48, OA52};

        assert_eq!(OA48::BITS, 48);
        assert_eq!(OA52::BITS, 52);
    }

    #[test]
    fn test_tte_alignment_helpers() {
        use crate::structures::tte::{Granule, Granule4KB, TTE64};

        type TTE = TTE64<Granule4KB, crate::structures::tte::OA48>;

        // Test is_aligned
        assert!(TTE::is_aligned(0x1000)); // 4KB aligned
        assert!(TTE::is_aligned(0x2000)); // 8KB aligned
        assert!(!TTE::is_aligned(0x1001)); // Not aligned
        assert!(!TTE::is_aligned(0xFFF)); // Not aligned

        // Test align_down
        assert_eq!(TTE::align_down(0x1FFF), 0x1000);
        assert_eq!(TTE::align_down(0x2000), 0x2000);
        assert_eq!(TTE::align_down(0x3001), 0x3000);

        // Test align_up
        assert_eq!(TTE::align_up(0x1000), 0x1000);
        assert_eq!(TTE::align_up(0x1001), 0x2000);
        assert_eq!(TTE::align_up(0x1FFF), 0x2000);
        assert_eq!(TTE::align_up(0x2000), 0x2000);
    }
}
