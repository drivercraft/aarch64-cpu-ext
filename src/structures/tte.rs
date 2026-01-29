//! # Translation Table Entry (TTE) for AArch64
//!
//! This module defines the Translation Table Entry (TTE) structure used in AArch64
//! architecture for virtual to physical address translation. It provides a type-safe
//! abstraction over the raw hardware representation of page table entries.
//!
//! ## Overview
//!
//! AArch64 uses multi-level page tables (up to 4 levels) to translate virtual
//! addresses to physical addresses. Each entry in the page table is represented
//! by a 64-bit TTE that can be either:
//!
//! - **Table entry**: Points to the next level of the page table
//! - **Block entry**: Directly maps a contiguous region of memory
//! - **Invalid entry**: Marks the entry as not valid
//!
//! ## Granule Support
//!
//! The module supports three granule sizes:
//! - **4KB**: Standard page size for most systems
//! - **16KB**: Alternative granule for certain ARM implementations
//! - **64KB**: Larger granule for improved TLB efficiency
//!
//! ## Output Address Sizes
//!
//! Two output address width configurations are supported:
//! - **48-bit**: Standard configuration for most systems (up to 256 TB physical memory)
//! - **52-bit**: Extended configuration for systems with larger physical memory (up to 4 PB)
//!
use core::marker::PhantomData;

use tock_registers::{LocalRegisterCopy, register_bitfields};

/// Trait defining granule (page table entry) size parameters.
///
/// A granule represents the minimum alignment and size unit for translation
/// table entries and page mappings in the AArch64 memory management system.
///
/// # Associated Constants
///
/// - `M`: Log2 of the granule size in bytes
/// - `SIZE`: The granule size in bytes (calculated as 2^M)
/// - `MASK`: Bit mask for granule alignment (calculated as (1 << M) - 1)
///
/// # Type Safety
///
/// This trait is used as a type parameter to enforce correct alignment and
/// size constraints at compile time when creating and manipulating TTEs.
pub trait Granule: Clone + Copy {
    /// Log2 of the granule size in bytes.
    ///
    /// For example:
    /// - 4KB granule: M = 12 (2^12 = 4096)
    /// - 16KB granule: M = 14 (2^14 = 16384)
    /// - 64KB granule: M = 16 (2^16 = 65536)
    const M: u32;

    /// The granule size in bytes (calculated as 2^M).
    const SIZE: usize = 2usize.pow(Self::M);

    /// Bit mask for granule alignment (calculated as (1 << M) - 1).
    ///
    /// This mask can be used to check if an address is properly aligned
    /// to the granule boundary: `(addr & MASK) == 0`.
    const MASK: u64 = (1u64 << Self::M) - 1; // Mask for alignment
}

/// 4KB granule marker type.
///
/// This type is used as a type parameter to indicate that a TTE uses the
/// 4KB granule configuration. With a 4KB granule, the page table structure
/// uses 9 bits at each level for indexing.
#[derive(Clone, Copy)]
pub struct Granule4KB {}

impl Granule for Granule4KB {
    const M: u32 = 12; // log2(4096) = 12
}

/// 16KB granule marker type.
///
/// This type is used as a type parameter to indicate that a TTE uses the
/// 16KB granule configuration. With a 16KB granule, the page table structure
/// uses 11 bits at levels 1-3 for indexing.
#[derive(Clone, Copy)]
pub struct Granule16KB {}

impl Granule for Granule16KB {
    const M: u32 = 14; // log2(16384) = 14
}

/// 64KB granule marker type.
///
/// This type is used as a type parameter to indicate that a TTE uses the
/// 64KB granule configuration. With a 64KB granule, the page table structure
/// uses different bit counts at each level (6, 13, 13 bits).
#[derive(Clone, Copy)]
pub struct Granule64KB {}

impl Granule for Granule64KB {
    const M: u32 = 16; // log2(65536) = 16
}

/// Trait defining output address (physical address) width parameters.
///
/// This trait specifies the number of bits available for the physical
/// address in a translation table entry.
///
/// # Associated Constants
///
/// - `BITS`: The number of bits for the output address (typically 48 or 52)
pub trait OA: Clone + Copy {
    /// The number of bits available for the output address.
    ///
    /// Common values:
    /// - 48 bits: Supports up to 256 TB of physical memory
    /// - 52 bits: Supports up to 4 PB of physical memory (requires ARMv8.4-LPA)
    const BITS: usize;
}

/// 48-bit output address marker type.
///
/// This type is used as a type parameter to indicate that a TTE uses the
/// 48-bit output address configuration, which is the standard configuration
/// for most AArch64 systems supporting up to 256 TB of physical memory.
#[derive(Clone, Copy)]
pub struct OA48 {}

impl OA for OA48 {
    const BITS: usize = 48; // 48-bit output address
}

/// 52-bit output address marker type.
///
/// This type is used as a type parameter to indicate that a TTE uses the
/// 52-bit output address configuration. This extended configuration requires
/// ARMv8.4-LPA (Large Physical Address) support and enables addressing of
/// up to 4 PB of physical memory.
#[derive(Clone, Copy)]
pub struct OA52 {}

impl OA for OA52 {
    const BITS: usize = 52; // 52-bit output address
}

/// Access permissions for Stage 1 translation using Direct permissions.
///
/// These permissions control read and write access for different privilege levels.
/// The exact behavior depends on the translation regime (single or two privilege levels).
///
/// Based on ARM DDI 0487K.a Table D8-49.
///
/// # Variants
///
/// - `PrivilegedReadWrite`: Read/write access for privileged level only (AP\[2:1\] = 0b00)
/// - `ReadWrite`: Read/write access for both privileged and unprivileged levels (AP\[2:1\] = 0b01)
/// - `PrivilegedReadOnly`: Read-only access for privileged level only (AP\[2:1\] = 0b10)
/// - `ReadOnly`: Read-only access for both privileged and unprivileged levels (AP\[2:1\] = 0b11)
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessPermission {
    /// Read/write access for privileged level only, no access for unprivileged
    /// AP\[2:1\] = 0b00 (when supporting two privilege levels)
    /// For single privilege level: Read/write access
    PrivilegedReadWrite = 0b00,

    /// Read/write access for both privileged and unprivileged levels
    /// AP\[2:1\] = 0b01
    ReadWrite = 0b01,

    /// Read-only access for privileged level only, no access for unprivileged
    /// AP\[2:1\] = 0b10 (when supporting two privilege levels)
    /// For single privilege level: Read-only access
    PrivilegedReadOnly = 0b10,

    /// Read-only access for both privileged and unprivileged levels
    /// AP\[2:1\] = 0b11
    ReadOnly = 0b11,
}

impl AccessPermission {
    /// Get the AP field value for the TTE
    pub const fn as_bits(self) -> u8 {
        self as u8
    }

    /// Create from AP bits
    pub const fn from_bits(bits: u8) -> Option<Self> {
        match bits & 0b11 {
            0b00 => Some(Self::PrivilegedReadWrite),
            0b01 => Some(Self::ReadWrite),
            0b10 => Some(Self::PrivilegedReadOnly),
            0b11 => Some(Self::ReadOnly),
            _ => None,
        }
    }

    /// Check if this permission allows unprivileged access
    pub const fn allows_unprivileged(self) -> bool {
        matches!(self, Self::ReadWrite | Self::ReadOnly)
    }

    /// Check if this permission allows write access at the privileged level
    pub const fn allows_privileged_write(self) -> bool {
        matches!(self, Self::PrivilegedReadWrite | Self::ReadWrite)
    }

    /// Check if this permission allows write access at the unprivileged level
    pub const fn allows_unprivileged_write(self) -> bool {
        matches!(self, Self::ReadWrite)
    }
}

/// Shareability attribute for memory regions.
///
/// Shareability controls the cache coherency behavior of memory accesses.
/// It determines how changes to memory are propagated between different
/// processors in a multi-core system.
///
/// # Variants
///
/// - `NonShareable`: Memory is not shared between processors; no coherency required
/// - `OuterShareable`: Memory is shared across multiple clusters; requires outer cache coherency
/// - `InnerShareable`: Memory is shared within a single cluster; requires inner cache coherency
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shareability {
    NonShareable,
    OuterShareable,
    InnerShareable,
}

register_bitfields![u64,
    /// Translation Table Entry for AArch64
    /// Based on ARMv8-A Architecture Reference Manual
    TTE64_REG [
        /// Valid bit - indicates if this entry is valid
        VALID OFFSET(0) NUMBITS(1) [
            Invalid = 0,
            Valid = 1
        ],

        /// Type bit for level 0, 1, and 2 entries
        /// Combined with VALID bit determines the entry type
        TYPE OFFSET(1) NUMBITS(1) [
            Block = 0,  // Block entry (when VALID=1)
            Table = 1   // Table entry (when VALID=1)
        ],

        /// Memory attributes index for MAIR_ELx
        ATTR_INDX OFFSET(2) NUMBITS(3) [],

        /// Non-secure bit
        NS OFFSET(5) NUMBITS(1) [
            Secure = 0,
            NonSecure = 1
        ],

        /// Access permission bits
        /// AP\[2:1\] for Stage 1 translation using Direct permissions
        /// Based on ARM DDI 0487K.a Table D8-49
        AP OFFSET(6) NUMBITS(2) [
            PrivilegedReadWrite = 0b00,  // Read/write for privileged level only
            ReadWrite = 0b01,            // Read/write for both privileged and unprivileged
            PrivilegedReadOnly = 0b10,   // Read-only for privileged level only
            ReadOnly = 0b11              // Read-only for both privileged and unprivileged
        ],

        /// Shareability field
        SH OFFSET(8) NUMBITS(2) [
            NonShareable = 0b00,
            OuterShareable = 0b10,
            InnerShareable = 0b11
        ],

        /// Access flag
        AF OFFSET(10) NUMBITS(1) [
            NotAccessed = 0,
            Accessed = 1
        ],

        /// Not global bit
        NG OFFSET(11) NUMBITS(1) [
            Global = 0,
            NotGlobal = 1
        ],

        ADDR OFFSET(12) NUMBITS(38) [],

        /// Dirty bit modifier (ARMv8.1+)
        DBM OFFSET(51) NUMBITS(1) [
            ReadOnly = 0,
            Writable = 1
        ],

        /// Contiguous bit
        CONTIG OFFSET(52) NUMBITS(1) [
            NotContiguous = 0,
            Contiguous = 1
        ],

        /// Privileged execute-never
        PXN OFFSET(53) NUMBITS(1) [
            ExecuteAllowed = 0,
            ExecuteNever = 1
        ],

        /// Execute-never or Unprivileged execute-never
        XN_UXN OFFSET(54) NUMBITS(1) [
            ExecuteAllowed = 0,
            ExecuteNever = 1
        ],

        /// Reserved for software use (bits 58:55)
        SW_RESERVED OFFSET(55) NUMBITS(4) []
    ]
];

/// Translation Table Entry (TTE) for AArch64.
///
/// This struct provides a type-safe interface to AArch64 translation table entries.
/// It uses phantom type parameters to enforce correct granule size and output
/// address width at compile time.
///
/// # Type Parameters
///
/// * `G`: Granule size marker type (e.g., `Granule4KB`, `Granule16KB`, `Granule64KB`)
/// * `O`: Output address width marker type (e.g., `OA48`, `OA52`)
///
#[derive(Clone, Copy)]
pub struct TTE64<G: Granule, O: OA> {
    reg: LocalRegisterCopy<u64, TTE64_REG::Register>,
    _marker: PhantomData<(G, O)>,
}

impl<G: Granule, O: OA> TTE64<G, O> {
    /// Creates a new TTE from a raw 64-bit value.
    ///
    /// This constructor is useful when you need to create a TTE from a raw
    /// value read from hardware or memory.
    ///
    /// # Arguments
    ///
    /// * `value` - The raw 64-bit TTE value
    pub const fn new(value: u64) -> Self {
        Self {
            reg: LocalRegisterCopy::new(value),
            _marker: PhantomData,
        }
    }

    /// Creates an invalid TTE (all bits set to zero).
    ///
    /// An invalid TTE has the valid bit cleared and maps no memory.
    /// This is useful for initializing page tables or unmapping regions.
    pub const fn invalid() -> Self {
        Self::new(0)
    }

    /// Creates a new table entry pointing to the next level of the page table.
    ///
    /// Table entries are used to build the multi-level page table structure.
    /// Each table entry points to the physical address of the next level table.
    ///
    /// # Arguments
    ///
    /// * `table_addr` - Physical address of the next level table (must be aligned to granule size)
    ///
    /// # Panics
    ///
    /// Panics if the address is not properly aligned to the granule size.
    pub fn new_table(table_addr: u64) -> Self {
        let mut tte = Self::new(0);

        tte.reg
            .modify(TTE64_REG::VALID::Valid + TTE64_REG::TYPE::Table + TTE64_REG::AF::Accessed);
        tte.set_address(table_addr);
        tte
    }

    /// Creates a new block entry mapping a contiguous memory region.
    ///
    /// Block entries provide a direct mapping of a contiguous memory region
    /// without requiring additional levels of page table lookup. This can
    /// improve TLB efficiency for large mappings.
    ///
    /// # Arguments
    ///
    /// * `block_addr` - Physical address of the block (must be aligned to the block size)
    ///
    /// # Panics
    ///
    /// Panics if the address is not properly aligned to the granule size.
    pub fn new_block(block_addr: u64) -> Self {
        let mut tte = Self::new(0);

        tte.reg
            .modify(TTE64_REG::VALID::Valid + TTE64_REG::TYPE::Block + TTE64_REG::AF::Accessed);
        tte.set_address(block_addr);
        tte
    }

    /// Returns the raw 64-bit value of this TTE.
    ///
    /// This is useful when you need to write the TTE to hardware or memory.
    ///
    /// # Returns
    ///
    /// The raw 64-bit TTE value
    pub fn get(&self) -> u64 {
        self.reg.get()
    }

    /// Checks if this TTE is valid.
    ///
    /// A valid TTE has the valid bit set and represents either a table
    /// entry or a block entry. Invalid TTEs map no memory.
    ///
    /// # Returns
    ///
    /// `true` if the TTE is valid, `false` otherwise
    pub fn is_valid(&self) -> bool {
        self.reg.is_set(TTE64_REG::VALID)
    }

    /// Sets the valid bit of this TTE.
    ///
    /// # Arguments
    ///
    /// * `val` - `true` to mark the entry as valid, `false` to mark as invalid
    pub fn set_is_valid(&mut self, val: bool) {
        if val {
            self.reg.modify(TTE64_REG::VALID::Valid);
        } else {
            self.reg.modify(TTE64_REG::VALID::Invalid);
        }
    }

    /// Checks if this TTE is a table entry.
    ///
    /// Table entries point to the next level of the page table structure.
    ///
    /// # Returns
    ///
    /// `true` if this is a valid table entry, `false` otherwise
    pub fn is_table(&self) -> bool {
        self.is_valid() && self.reg.is_set(TTE64_REG::TYPE)
    }

    /// Checks if this TTE is a block entry.
    ///
    /// Block entries directly map a contiguous memory region without
    /// requiring additional page table lookups.
    ///
    /// # Returns
    ///
    /// `true` if this is a valid block entry, `false` otherwise
    pub fn is_block(&self) -> bool {
        self.is_valid() && !self.reg.is_set(TTE64_REG::TYPE)
    }

    /// Sets this TTE to be a table entry.
    ///
    /// This marks the entry as a table descriptor type. The entry must
    /// already be valid.
    pub fn set_is_table(&mut self) {
        self.reg.modify(TTE64_REG::TYPE::Table);
    }

    /// Sets this TTE to be a block entry.
    ///
    /// This marks the entry as a block descriptor type. The entry must
    /// already be valid.
    pub fn set_is_block(&mut self) {
        self.reg.modify(TTE64_REG::TYPE::Block);
    }

    /// Sets the output (physical) address for this TTE.
    ///
    /// The address must be properly aligned according to the granule size
    /// and must fit within the output address width.
    ///
    /// # Arguments
    ///
    /// * `addr` - The physical address to set
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The address is not aligned to the granule size
    /// - The address exceeds the output address width
    pub fn set_address(&mut self, addr: u64) {
        assert!(
            addr & G::MASK == 0,
            "Address must be aligned to granule size"
        );
        assert!(
            addr < (1u64 << O::BITS),
            "Address exceeds output address width"
        );
        let val = addr >> TTE64_REG::ADDR.shift; // Shift to align with TTE address bits
        self.reg.modify(TTE64_REG::ADDR.val(val));
    }

    /// Gets the output (physical) address from this TTE.
    ///
    /// This extracts the address bits from the TTE and reconstructs the
    /// physical address. For table entries, this returns the address of
    /// the next-level table. For block entries, this returns the base
    /// address of the mapped block.
    ///
    /// # Returns
    ///
    /// The physical address (0 if the TTE is invalid)
    ///
    /// # Note
    ///
    /// This method returns 0 for invalid TTEs. Check `is_valid()` first
    /// if you need to distinguish between invalid entries and valid
    /// entries at address 0.
    pub fn address(&self) -> u64 {
        if !self.is_valid() {
            return 0;
        }

        let raw_value = self.reg.get();
        let m = G::M; // granule size log2 (12, 14, or 16)

        let bit_start = m;
        let bit_end =

        // Handle 52-bit output address extension
        if O::BITS == 52 && (G::M == 12 || G::M == 14) {
            50
        } else {
            48
        };
        let mask = ((1u64 << (bit_end - bit_start + 1)) - 1) << bit_start;
        raw_value & mask
    }

    /// Gets the output address for a block entry at a specific page table level.
    ///
    /// This method calculates the base address of a block mapping considering
    /// the block size at the specified level. This is useful for extracting
    /// the correct base address from block entries, as block entries store
    /// only the upper address bits (the lower bits are implied zeros).
    ///
    /// # Arguments
    ///
    /// * `level` - The page table level (0-3)
    ///
    /// # Returns
    ///
    /// The physical address of the block (for table entries, returns the
    /// same as `address()`)
    ///
    /// # Panics
    ///
    /// Panics if the granule size and level combination is invalid.
    pub fn address_with_page_level(&self, level: usize) -> u64 {
        if self.is_table() {
            return self.address();
        }
        let raw_addr = self.reg.get();
        let n = match (G::M, level) {
            (12, 0) => 39,
            (12, 1) => 30,
            (12, 2) => 21,
            (14, 1) => 36,
            (14, 2) => 25,
            (16, 1) => 42,
            (16, 2) => 29,
            _ => panic!("Invalid granule size or level combination"),
        };

        let bit_start = n;
        // 4KB and 16KB granules, 52-bit OA
        let bit_end = if O::BITS == 52 && (G::M == 12 || G::M == 14) {
            50
        } else {
            48
        };
        let mask = ((1u64 << (bit_end - bit_start + 1)) - 1) << bit_start;
        raw_addr & mask
    }

    /// Checks if the access flag is set.
    ///
    /// The access flag is set by hardware on the first access to a page.
    /// It can be used by software to implement page aging algorithms.
    ///
    /// # Returns
    ///
    /// `true` if the access flag is set, `false` otherwise
    pub fn is_accessed(&self) -> bool {
        self.reg.is_set(TTE64_REG::AF)
    }

    /// Gets the memory attribute index.
    ///
    /// The attribute index selects a memory attribute configuration from
    /// the MAIR_EL1 (Memory Attribute Indirection Register) or MAIR_EL2/EL3.
    ///
    /// # Returns
    ///
    /// The attribute index (0-7)
    pub fn attr_index(&self) -> u64 {
        self.reg.read(TTE64_REG::ATTR_INDX)
    }

    /// Sets the memory attribute index.
    ///
    /// The attribute index selects a memory attribute configuration from
    /// the MAIR_ELx registers. The index must be less than 8.
    ///
    /// # Arguments
    ///
    /// * `index` - The attribute index (0-7)
    ///
    /// # Panics
    ///
    /// Panics if the index is >= 8.
    pub fn set_attr_index(&mut self, index: u64) {
        assert!(index < 8, "Attribute index must be less than 8");
        self.reg.modify(TTE64_REG::ATTR_INDX.val(index));
    }

    /// Check if this TTE allows execution (reads XN/UXN bit at \[54\])
    ///
    /// Returns `true` if execution is allowed, `false` if Execute Never is set.
    /// The specific meaning depends on the translation regime - see `set_executable()` for details.
    pub fn is_executable(&self) -> bool {
        !self.reg.is_set(TTE64_REG::XN_UXN)
    }

    /// Set the execution permission (controls XN/UXN bit at \[54\])
    ///
    /// The meaning of this bit depends on the translation regime:
    /// - **Single privilege level**: Execute-never (XN) - controls execution for the single privilege level
    /// - **Two privilege levels**: Unprivileged Execute-never (UXN) - controls execution at unprivileged level
    /// - **EL1&0 regime with HCR_EL2.{NV, NV1} = {1, 1}**: Functions as Privileged Execute-never (PXN) when UXN effective value is 0
    ///
    /// When `val` is:
    /// - `true`: Allows execution (XN/UXN = 0)
    /// - `false`: Blocks execution (XN/UXN = 1, Execute Never)
    ///
    /// See ARM DDI 0487K.a "Stage 1 instruction execution using Direct permissions"
    pub fn set_executable(&mut self, val: bool) {
        if val {
            self.reg.modify(TTE64_REG::XN_UXN::ExecuteAllowed);
        } else {
            self.reg.modify(TTE64_REG::XN_UXN::ExecuteNever);
        }
    }

    /// Check if this TTE allows privileged execution (reads PXN bit at \[53\])
    ///
    /// Returns `true` if privileged execution is allowed, `false` if Privileged Execute Never is set.
    /// The specific meaning depends on the translation regime - see `set_privileged_executable()` for details.
    pub fn is_privileged_executable(&self) -> bool {
        !self.reg.is_set(TTE64_REG::PXN)
    }

    /// Set the privileged execution permission (controls PXN bit at \[53\])
    ///
    /// The meaning of this bit depends on the translation regime:
    /// - **Single privilege level**: RES0 (Reserved, should be 0)
    /// - **Two privilege levels**: Privileged Execute-never (PXN) - controls execution at privileged level
    /// - **EL1&0 regime with HCR_EL2.{NV, NV1} = {1, 1}**: RES0 (Reserved, should be 0)
    ///
    /// When `val` is:
    /// - `true`: Allows privileged execution (PXN = 0) - only valid for two privilege level regimes
    /// - `false`: Blocks privileged execution (PXN = 1, Execute Never) - only valid for two privilege level regimes
    ///
    /// **Note**: In single privilege level regimes or specific nested virtualization configurations,
    /// this bit is reserved and should be set to 0.
    ///
    /// See ARM DDI 0487K.a "Stage 1 instruction execution using Direct permissions"
    pub fn set_privileged_executable(&mut self, val: bool) {
        if val {
            self.reg.modify(TTE64_REG::PXN::ExecuteAllowed);
        } else {
            self.reg.modify(TTE64_REG::PXN::ExecuteNever);
        }
    }

    /// Gets the access permissions.
    ///
    /// Returns the current access permission setting for this TTE.
    ///
    /// # Returns
    ///
    /// The current access permission
    pub fn access_permission(&self) -> AccessPermission {
        AccessPermission::from_bits(self.reg.read(TTE64_REG::AP) as _).unwrap()
    }

    /// Sets the access permissions.
    ///
    /// # Arguments
    ///
    /// * `permission` - The access permission to set
    pub fn set_access_permission(&mut self, permission: AccessPermission) {
        self.reg
            .modify(TTE64_REG::AP.val(permission.as_bits() as u64));
    }

    /// Gets the shareability attribute.
    ///
    /// Returns the current shareability setting which determines how
    /// cache coherency is managed for this memory region.
    ///
    /// # Returns
    ///
    /// The shareability attribute
    pub fn shareability(&self) -> Shareability {
        match self.reg.read_as_enum(TTE64_REG::SH) {
            Some(TTE64_REG::SH::Value::NonShareable) => Shareability::NonShareable,
            Some(TTE64_REG::SH::Value::OuterShareable) => Shareability::OuterShareable,
            Some(TTE64_REG::SH::Value::InnerShareable) => Shareability::InnerShareable,
            None => unreachable!("invalid value"),
        }
    }

    /// Sets the shareability attribute.
    ///
    /// # Arguments
    ///
    /// * `shareability` - The shareability attribute to set
    pub fn set_shareability(&mut self, shareability: Shareability) {
        self.reg.modify(match shareability {
            Shareability::NonShareable => TTE64_REG::SH::NonShareable,
            Shareability::OuterShareable => TTE64_REG::SH::OuterShareable,
            Shareability::InnerShareable => TTE64_REG::SH::InnerShareable,
        });
    }

    /// Sets the access flag.
    ///
    /// Marks this entry as accessed. This is typically set by hardware on
    /// the first access, but can also be set manually.
    pub fn set_access(&mut self) {
        self.reg.modify(TTE64_REG::AF::Accessed);
    }

    /// Clears the access flag.
    ///
    /// Marks this entry as not accessed. This can be useful for page
    /// aging algorithms or for detecting unused pages.
    pub fn clear_access(&mut self) {
        self.reg.modify(TTE64_REG::AF::NotAccessed);
    }

    /// Checks if the contiguous bit is set.
    ///
    /// The contiguous bit hints that adjacent entries are part of a contiguous
    /// mapping, which can allow hardware optimizations.
    ///
    /// # Returns
    ///
    /// `true` if the contiguous bit is set, `false` otherwise
    pub fn is_contiguous(&self) -> bool {
        self.reg.is_set(TTE64_REG::CONTIG)
    }

    /// Sets the contiguous bit.
    ///
    /// Marks this entry as part of a contiguous mapping. This can improve
    /// TLB efficiency when adjacent entries form a contiguous region.
    pub fn set_contiguous(&mut self) {
        self.reg.modify(TTE64_REG::CONTIG::Contiguous);
    }

    /// Checks if this is a global mapping.
    ///
    /// Global mappings are not flushed by ASID-based TLB invalidations
    /// and are shared across all address spaces.
    ///
    /// # Returns
    ///
    /// `true` if this is a global mapping, `false` if process-specific
    pub fn is_global(&self) -> bool {
        !self.reg.is_set(TTE64_REG::NG)
    }

    /// Sets the not-global bit.
    ///
    /// Marks this mapping as process-specific (non-global). Process-specific
    /// mappings are included in ASID-based TLB invalidations.
    pub fn set_not_global(&mut self) {
        self.reg.modify(TTE64_REG::NG::NotGlobal);
    }

    /// Checks if the dirty bit modifier is set.
    ///
    /// The dirty bit modifier (DBM) is part of the ARMv8.1 hardware
    /// page table update feature. When set, it indicates that the page
    /// is writable and dirty tracking is enabled.
    ///
    /// # Returns
    ///
    /// `true` if the dirty bit modifier is set, `false` otherwise
    pub fn is_dirty_writable(&self) -> bool {
        self.reg.is_set(TTE64_REG::DBM)
    }

    /// Gets the software reserved bits.
    ///
    /// Bits \[58:55\] are reserved for software use and can be used for
    /// software-specific metadata.
    ///
    /// # Returns
    ///
    /// The software reserved bits (0-15)
    pub fn sw_reserved(&self) -> u64 {
        self.reg.read(TTE64_REG::SW_RESERVED)
    }

    /// Sets the software reserved bits.
    ///
    /// Bits \[58:55\] are reserved for software use and can be used for
    /// software-specific metadata.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to set (only lower 4 bits are used)
    pub fn set_sw_reserved(&mut self, value: u64) {
        self.reg.modify(TTE64_REG::SW_RESERVED.val(value & 0xF));
    }
}

// Convenient type aliases for common configurations

/// TTE with 4KB granule and 48-bit output addresses.
///
/// This is the most common configuration for AArch64 systems, providing
/// 4KB page size support with up to 256 TB of physical addressable memory.
pub type TTE4K48 = TTE64<Granule4KB, OA48>;

/// TTE with 4KB granule and 52-bit output addresses.
///
/// This configuration provides 4KB page size support with extended 52-bit
/// physical addresses, enabling up to 4 PB of addressable memory.
/// Requires ARMv8.4-LPA support.
pub type TTE4K52 = TTE64<Granule4KB, OA52>;

/// TTE with 16KB granule and 48-bit output addresses.
///
/// This configuration provides 16KB page size support with up to 256 TB
/// of physical addressable memory. Useful for systems that benefit from
/// larger page sizes.
pub type TTE16K48 = TTE64<Granule16KB, OA48>;

/// TTE with 16KB granule and 52-bit output addresses.
///
/// This configuration provides 16KB page size support with extended 52-bit
/// physical addresses, enabling up to 4 PB of addressable memory.
/// Requires ARMv8.4-LPA support.
pub type TTE16K52 = TTE64<Granule16KB, OA52>;

/// TTE with 64KB granule and 48-bit output addresses.
///
/// This configuration provides 64KB page size support with up to 256 TB
/// of physical addressable memory. The large page size can improve TLB
/// efficiency for memory-intensive workloads.
pub type TTE64K48 = TTE64<Granule64KB, OA48>;

/// TTE with 64KB granule and 52-bit output addresses.
///
/// This configuration provides 64KB page size support with extended 52-bit
/// physical addresses, enabling up to 4 PB of addressable memory.
/// Requires ARMv8.4-LPA support.
pub type TTE64K52 = TTE64<Granule64KB, OA52>;

/// Constants for block sizes at different page table levels.
///
/// This module defines the block sizes for each granule size at each
/// level of the page table hierarchy. Block entries can be used at levels
/// 0, 1, or 2 to map large contiguous regions without requiring additional
/// page table levels.
pub mod block_sizes {
    /// Block sizes for 4KB granule
    pub mod granule_4k {
        pub const LEVEL1_BLOCK_SIZE: usize = 1024 * 1024 * 1024; // 1GB
        pub const LEVEL2_BLOCK_SIZE: usize = 2 * 1024 * 1024; // 2MB
        pub const LEVEL3_PAGE_SIZE: usize = 4 * 1024; // 4KB
    }

    /// Block sizes for 16KB granule
    pub mod granule_16k {
        pub const LEVEL1_BLOCK_SIZE: usize = 64 * 1024 * 1024 * 1024; // 64GB
        pub const LEVEL2_BLOCK_SIZE: usize = 32 * 1024 * 1024; // 32MB
        pub const LEVEL3_PAGE_SIZE: usize = 16 * 1024; // 16KB
    }

    /// Block sizes for 64KB granule
    pub mod granule_64k {
        pub const LEVEL1_BLOCK_SIZE: usize = 4 * 1024 * 1024 * 1024; // 4TB (level0)
        pub const LEVEL2_BLOCK_SIZE: usize = 512 * 1024 * 1024; // 512MB
        pub const LEVEL3_PAGE_SIZE: usize = 64 * 1024; // 64KB
    }
}

/// Helper functions for address calculations.
impl<G: Granule, O: OA> TTE64<G, O> {
    /// Calculates the page table index for a virtual address at a given level.
    ///
    /// This function extracts the appropriate bits from a virtual address to
    /// index into a page table at the specified level. The number of bits used
    /// and their position depends on the granule size and the level.
    ///
    /// # Arguments
    ///
    /// * `va` - The virtual address
    /// * `level` - The page table level (0-3)
    ///
    /// # Returns
    ///
    /// The index into the page table at the specified level
    ///
    /// # Panics
    ///
    /// Panics if the granule size and level combination is invalid.
    pub fn calculate_index(va: u64, level: usize) -> usize {
        match (G::M, level) {
            // 4KB granule
            (12, 0) => ((va >> 39) & 0x1FF) as usize, // 9 bits
            (12, 1) => ((va >> 30) & 0x1FF) as usize, // 9 bits
            (12, 2) => ((va >> 21) & 0x1FF) as usize, // 9 bits
            (12, 3) => ((va >> 12) & 0x1FF) as usize, // 9 bits
            // 16KB granule
            (14, 0) => ((va >> 47) & 0x1) as usize,   // 1 bit
            (14, 1) => ((va >> 36) & 0x7FF) as usize, // 11 bits
            (14, 2) => ((va >> 25) & 0x7FF) as usize, // 11 bits
            (14, 3) => ((va >> 14) & 0x7FF) as usize, // 11 bits
            // 64KB granule
            (16, 1) => ((va >> 42) & 0x3F) as usize, // 6 bits
            (16, 2) => ((va >> 29) & 0x1FFF) as usize, // 13 bits
            (16, 3) => ((va >> 16) & 0x1FFF) as usize, // 13 bits
            _ => panic!("Invalid granule size or level combination"),
        }
    }

    /// Checks if an address is aligned to the granule boundary.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to check
    ///
    /// # Returns
    ///
    /// `true` if the address is aligned, `false` otherwise
    pub fn is_aligned(addr: u64) -> bool {
        (addr & G::MASK) == 0
    }

    /// Aligns an address down to the granule boundary.
    ///
    /// This rounds the address down to the nearest granule boundary.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to align
    ///
    /// # Returns
    ///
    /// The aligned address
    pub fn align_down(addr: u64) -> u64 {
        addr & !G::MASK
    }

    /// Aligns an address up to the granule boundary.
    ///
    /// This rounds the address up to the nearest granule boundary.
    /// If the address is already aligned, it is returned unchanged.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to align
    ///
    /// # Returns
    ///
    /// The aligned address
    pub fn align_up(addr: u64) -> u64 {
        (addr + G::MASK) & !G::MASK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_extraction_4k_48bit() {
        // Test 4KB granule with 48-bit output address
        type TTE = TTE64<Granule4KB, OA48>;

        // Test table descriptor
        let table_addr = 0x1000_0000_1000; // 48-bit address aligned to 4KB
        let tte_table = TTE::new_table(table_addr);
        assert_eq!(tte_table.address(), table_addr);

        // Test block descriptor
        let block = 2 * 1024 * 1024; // 2MB
        let block_addr = 0x2000_0000_1000 + block; // 48-bit address aligned to 4KB 
        let tte_block = TTE::new_block(block_addr);
        assert_eq!(
            tte_block.address_with_page_level(2),
            0x2000_0000_0000 + block
        );
    }

    #[test]
    fn test_address_extraction_4k_52bit() {
        // Test 4KB granule with 52-bit output address
        type TTE = TTE64<Granule4KB, OA52>;

        let table_addr = (1 << 50) - 0x1000; // 52-bit address with high bits
        let tte_table = TTE::new_table(table_addr);
        let read_addr = tte_table.address();
        assert_eq!(
            read_addr, table_addr,
            "want {:#x} != read {:#x} address mismatch",
            table_addr, read_addr
        );
    }

    #[test]
    fn test_address_extraction_16k_48bit() {
        // Test 16KB granule with 48-bit output address
        type TTE = TTE64<Granule16KB, OA48>;

        // Test table descriptor - must be aligned to 16KB boundary
        let table_addr = (1 << 47) + 16 * 1024; // 48-bit address aligned to 16KB
        let tte_table = TTE::new_table(table_addr);
        let read = tte_table.address();
        assert_eq!(
            table_addr, read,
            "want {:#x} != read {:#x} address mismatch",
            table_addr, read
        );

        // Test block descriptor
        let block_addr = 0x2000_0000_0000; // 48-bit address aligned to 16KB
        let tte_block = TTE::new_block(block_addr);
        assert_eq!(tte_block.address(), block_addr);
    }

    #[test]
    fn test_address_extraction_16k_52bit() {
        // Test 16KB granule with 52-bit output address
        type TTE = TTE64<Granule16KB, OA52>;

        // Test with high address bits
        let table_addr = (1 << 50) - 0x4000; // 52-bit address with high bits, aligned to 16KB
        let tte_table = TTE::new_table(table_addr); // Base address aligned to 16KB

        assert_eq!(tte_table.address(), table_addr);
    }

    #[test]
    fn test_address_extraction_64k_48bit() {
        // Test 64KB granule with 48-bit output address
        type TTE = TTE64<Granule64KB, OA48>;

        // Test table descriptor - must be aligned to 64KB boundary
        let table_addr = 0x1000_0001_0000; // 48-bit address aligned to 64KB
        let tte_table = TTE::new_table(table_addr);
        assert_eq!(tte_table.address(), table_addr);

        // Test block descriptor
        let block_addr = 0x2000_0002_0000; // 48-bit address aligned to 64KB
        let tte_block = TTE::new_block(block_addr);
        assert_eq!(tte_block.address(), block_addr);
    }

    #[test]
    fn test_address_extraction_64k_52bit() {
        // Test 64KB granule with 52-bit output address
        type TTE = TTE64<Granule64KB, OA52>;

        let table_addr = 0xf00_1001_0000u64; // 52-bit address with high bits, aligned to 64KB
        let tte_table = TTE::new_table(table_addr); // Base address aligned to 64KB

        assert_eq!(
            table_addr,
            tte_table.address(),
            "want {:#x} != read {:#x} address mismatch",
            table_addr,
            tte_table.address()
        );
    }

    #[test]
    fn test_invalid_tte_address() {
        // Test that invalid TTEs return 0 address
        type TTE = TTE64<Granule4KB, OA48>;

        let tte_invalid = TTE::invalid();
        assert_eq!(tte_invalid.address(), 0);
        assert!(!tte_invalid.is_valid());
    }

    #[test]
    fn test_granule_constants() {
        // Test that granule constants match expected values for m calculation
        assert_eq!(Granule4KB::M, 12); // log2(4096) = 12
        assert_eq!(Granule16KB::M, 14); // log2(16384) = 14  
        assert_eq!(Granule64KB::M, 16); // log2(65536) = 16

        // Test that granule sizes are correct
        assert_eq!(Granule4KB::SIZE, 4096);
        assert_eq!(Granule16KB::SIZE, 16384);
        assert_eq!(Granule64KB::SIZE, 65536);

        // Test that masks are correct for alignment
        assert_eq!(Granule4KB::MASK, 0xFFF);
        assert_eq!(Granule16KB::MASK, 0x3FFF);
        assert_eq!(Granule64KB::MASK, 0xFFFF);
    }
}
