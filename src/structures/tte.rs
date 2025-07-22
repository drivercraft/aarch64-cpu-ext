use core::marker::PhantomData;

/// This module defines the Translation Table Entry (TTE) structure used in AArch64 architecture.
use tock_registers::{LocalRegisterCopy, register_bitfields};

pub trait Granule: Clone + Copy {
    const SIZE: usize;
    const SHIFT: usize;
    const MASK: u64;
}

#[derive(Clone, Copy)]
pub struct Granule4KB {}

impl Granule for Granule4KB {
    const SIZE: usize = 4096; // 4KB granule size
    const SHIFT: usize = 12; // log2(4096) = 12
    const MASK: u64 = 0xFFF; // 4KB alignment mask
}

#[derive(Clone, Copy)]
pub struct Granule16KB {}

impl Granule for Granule16KB {
    const SIZE: usize = 16 * 1024; // 16KB granule size
    const SHIFT: usize = 14; // log2(16384) = 14
    const MASK: u64 = 0x3FFF; // 16KB alignment mask
}

#[derive(Clone, Copy)]
pub struct Granule64KB {}

impl Granule for Granule64KB {
    const SIZE: usize = 64 * 1024; // 64KB granule size
    const SHIFT: usize = 16; // log2(65536) = 16
    const MASK: u64 = 0xFFFF; // 64KB alignment mask
}

pub trait OA: Clone + Copy {
    const BITS: usize;
    const OUTPUT_ADDR_BITS: usize;
    const OUTPUT_ADDR_MASK: u64;
}

#[derive(Clone, Copy)]
pub struct OA48 {}

impl OA for OA48 {
    const BITS: usize = 48; // 48-bit output address
    const OUTPUT_ADDR_BITS: usize = 36; // bits [47:12] = 36 bits
    const OUTPUT_ADDR_MASK: u64 = (1u64 << 36) - 1; // mask for 36 bits
}

#[derive(Clone, Copy)]
pub struct OA52 {}

impl OA for OA52 {
    const BITS: usize = 52; // 52-bit output address
    const OUTPUT_ADDR_BITS: usize = 40; // bits [51:12] = 40 bits
    const OUTPUT_ADDR_MASK: u64 = (1u64 << 40) - 1; // mask for 40 bits
}

/// Access permission constants for TTE64
pub mod access_permissions {
    pub const RW_EL1: u8 = 0b00; // Read/write at EL1, no access at EL0
    pub const RW_EL1_EL0: u8 = 0b01; // Read/write at EL1 and EL0
    pub const RO_EL1: u8 = 0b10; // Read-only at EL1, no access at EL0
    pub const RO_EL1_EL0: u8 = 0b11; // Read-only at EL1 and EL0
}

/// Shareability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shareability {
    NonShareable = 0b00,
    OuterShareable = 0b10,
    InnerShareable = 0b11,
}

/// Configuration for creating block entries
#[derive(Debug, Clone, Copy)]
pub struct BlockConfig {
    pub attr_index: u8,
    pub access_permissions: u8,
    pub shareability: Shareability,
    pub executable: bool,
    pub privileged_executable: bool,
    pub contiguous: bool,
    pub not_global: bool,
}

impl Default for BlockConfig {
    fn default() -> Self {
        Self {
            attr_index: 0,
            access_permissions: access_permissions::RW_EL1,
            shareability: Shareability::InnerShareable,
            executable: false,
            privileged_executable: false,
            contiguous: false,
            not_global: false,
        }
    }
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
        AP OFFSET(6) NUMBITS(2) [
            RW_EL1 = 0b00,      // Read/write at EL1, no access at EL0
            RW_EL1_EL0 = 0b01,  // Read/write at EL1 and EL0
            RO_EL1 = 0b10,      // Read-only at EL1, no access at EL0
            RO_EL1_EL0 = 0b11   // Read-only at EL1 and EL0
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

        /// Output address bits [47:12] for 48-bit PA
        /// For table entries: points to next level table
        /// For block entries: block base address
        OUTPUT_ADDR OFFSET(12) NUMBITS(36) [],

        /// Output address bits [51:48] for 52-bit PA (ARMv8.2+)
        /// Only used when 52-bit PA is enabled
        OUTPUT_ADDR_HIGH OFFSET(48) NUMBITS(4) [],

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

#[derive(Debug, Clone, Copy)]
pub struct TTE64<G: Granule, O: OA> {
    reg: LocalRegisterCopy<u64, TTE64_REG::Register>,
    _marker: PhantomData<(G, O)>,
}

impl<G: Granule, O: OA> TTE64<G, O> {
    /// Create a new TTE64 from a raw u64 value
    pub const fn new(value: u64) -> Self {
        Self {
            reg: LocalRegisterCopy::new(value),
            _marker: PhantomData,
        }
    }

    /// Create an invalid TTE (all zeros)
    pub const fn invalid() -> Self {
        Self::new(0)
    }

    /// Create a table entry with more convenient parameters
    pub fn new_table(table_addr: u64, attr_index: u8, ns: bool, ap: u8) -> Self {
        assert!(
            table_addr & G::MASK == 0,
            "Table address must be aligned to granule size"
        );
        assert!(
            table_addr < (1u64 << O::BITS),
            "Table address exceeds output address width"
        );

        let mut tte = Self::new(0);
        let addr_low = (table_addr >> G::SHIFT) & O::OUTPUT_ADDR_MASK;

        tte.reg.modify(
            TTE64_REG::VALID::Valid
                + TTE64_REG::TYPE::Table
                + TTE64_REG::OUTPUT_ADDR.val(addr_low & ((1u64 << 36) - 1))
                + TTE64_REG::ATTR_INDX.val(attr_index as u64)
                + TTE64_REG::NS.val(if ns { 1 } else { 0 })
                + TTE64_REG::AP.val(ap as u64),
        );

        // For 52-bit addresses, set the high bits
        if O::BITS == 52 {
            let addr_high = (addr_low >> 36) & 0xF;
            tte.reg.modify(TTE64_REG::OUTPUT_ADDR_HIGH.val(addr_high));
        }

        tte
    }

    /// Create a block entry with BlockConfig
    pub fn new_block(block_addr: u64, config: BlockConfig) -> Self {
        assert!(
            block_addr & G::MASK == 0,
            "Block address must be aligned to granule size"
        );
        assert!(
            block_addr < (1u64 << O::BITS),
            "Block address exceeds output address width"
        );

        let mut tte = Self::new(0);
        let addr_low = (block_addr >> G::SHIFT) & O::OUTPUT_ADDR_MASK;

        tte.reg.modify(
            TTE64_REG::VALID::Valid
                + TTE64_REG::TYPE::Block
                + TTE64_REG::OUTPUT_ADDR.val(addr_low & ((1u64 << 36) - 1))
                + TTE64_REG::ATTR_INDX.val(config.attr_index as u64)
                + TTE64_REG::AP.val(config.access_permissions as u64)
                + TTE64_REG::SH.val(config.shareability as u64)
                + TTE64_REG::AF::Accessed
                + TTE64_REG::XN_UXN.val(if config.executable { 0 } else { 1 })
                + TTE64_REG::PXN.val(if config.privileged_executable { 0 } else { 1 })
                + TTE64_REG::CONTIG.val(if config.contiguous { 1 } else { 0 })
                + TTE64_REG::NG.val(if config.not_global { 1 } else { 0 }),
        );

        // For 52-bit addresses, set the high bits
        if O::BITS == 52 {
            let addr_high = (addr_low >> 36) & 0xF;
            tte.reg.modify(TTE64_REG::OUTPUT_ADDR_HIGH.val(addr_high));
        }

        tte
    }

    /// Create a table entry pointing to the next level page table
    pub fn table(table_addr: u64, attr_index: u8) -> Self {
        assert!(
            table_addr & G::MASK == 0,
            "Table address must be aligned to granule size"
        );
        assert!(
            table_addr < (1u64 << O::BITS),
            "Table address exceeds output address width"
        );

        let mut tte = Self::new(0);
        let addr_low = (table_addr >> G::SHIFT) & O::OUTPUT_ADDR_MASK;

        tte.reg.modify(
            TTE64_REG::VALID::Valid
                + TTE64_REG::TYPE::Table
                + TTE64_REG::OUTPUT_ADDR.val(addr_low & ((1u64 << 36) - 1))
                + TTE64_REG::ATTR_INDX.val(attr_index as u64),
        );

        // For 52-bit addresses, set the high bits
        if O::BITS == 52 {
            let addr_high = (addr_low >> 36) & 0xF;
            tte.reg.modify(TTE64_REG::OUTPUT_ADDR_HIGH.val(addr_high));
        }

        tte
    }

    /// Create a block entry mapping a large page
    pub fn block(
        block_addr: u64,
        attr_index: u8,
        access_perms: u8,
        shareability: u8,
        executable: bool,
        privileged_executable: bool,
    ) -> Self {
        assert!(
            block_addr & G::MASK == 0,
            "Block address must be aligned to granule size"
        );
        assert!(
            block_addr < (1u64 << O::BITS),
            "Block address exceeds output address width"
        );

        let mut tte = Self::new(0);
        let addr_low = (block_addr >> G::SHIFT) & O::OUTPUT_ADDR_MASK;

        tte.reg.modify(
            TTE64_REG::VALID::Valid
                + TTE64_REG::TYPE::Block
                + TTE64_REG::OUTPUT_ADDR.val(addr_low & ((1u64 << 36) - 1))
                + TTE64_REG::ATTR_INDX.val(attr_index as u64)
                + TTE64_REG::AP.val(access_perms as u64)
                + TTE64_REG::SH.val(shareability as u64)
                + TTE64_REG::AF::Accessed
                + TTE64_REG::XN_UXN.val(if executable { 0 } else { 1 })
                + TTE64_REG::PXN.val(if privileged_executable { 0 } else { 1 }),
        );

        // For 52-bit addresses, set the high bits
        if O::BITS == 52 {
            let addr_high = (addr_low >> 36) & 0xF;
            tte.reg.modify(TTE64_REG::OUTPUT_ADDR_HIGH.val(addr_high));
        }

        tte
    }

    /// Get the raw u64 value
    pub fn get(&self) -> u64 {
        self.reg.get()
    }

    /// Check if this TTE is valid
    pub fn is_valid(&self) -> bool {
        self.reg.is_set(TTE64_REG::VALID)
    }

    /// Check if this TTE is a table entry (vs block entry)
    pub fn is_table(&self) -> bool {
        self.is_valid() && self.reg.is_set(TTE64_REG::TYPE)
    }

    /// Check if this TTE is a block entry (vs table entry)
    pub fn is_block(&self) -> bool {
        self.is_valid() && !self.reg.is_set(TTE64_REG::TYPE)
    }

    /// Get the output address (physical address) from this TTE
    /// This extracts the address bits and reconstructs the physical address
    pub fn output_address(&self) -> u64 {
        let addr_low = self.reg.read(TTE64_REG::OUTPUT_ADDR);
        let addr = if O::BITS == 52 {
            // For 52-bit addresses, combine low and high bits
            let addr_high = self.reg.read(TTE64_REG::OUTPUT_ADDR_HIGH);
            (addr_high << 36) | addr_low
        } else {
            // For 48-bit addresses, only use low bits
            addr_low
        };
        addr << G::SHIFT
    }

    /// Check if this TTE has the access flag set
    pub fn is_accessed(&self) -> bool {
        self.reg.is_set(TTE64_REG::AF)
    }

    /// Get the memory attribute index
    pub fn attr_index(&self) -> u64 {
        self.reg.read(TTE64_REG::ATTR_INDX)
    }

    /// Check if this TTE allows execution
    pub fn is_executable(&self) -> bool {
        !self.reg.is_set(TTE64_REG::XN_UXN)
    }

    /// Check if this TTE allows privileged execution
    pub fn is_privileged_executable(&self) -> bool {
        !self.reg.is_set(TTE64_REG::PXN)
    }

    /// Get access permissions
    pub fn access_permissions(&self) -> u64 {
        self.reg.read(TTE64_REG::AP)
    }

    /// Get shareability attributes
    pub fn shareability(&self) -> u64 {
        self.reg.read(TTE64_REG::SH)
    }

    /// Set the access flag
    pub fn set_accessed(&mut self) {
        self.reg.modify(TTE64_REG::AF::Accessed);
    }

    /// Clear the access flag
    pub fn clear_accessed(&mut self) {
        self.reg.modify(TTE64_REG::AF::NotAccessed);
    }

    /// Check if the contiguous bit is set
    pub fn is_contiguous(&self) -> bool {
        self.reg.is_set(TTE64_REG::CONTIG)
    }

    /// Set the contiguous bit
    pub fn set_contiguous(&mut self) {
        self.reg.modify(TTE64_REG::CONTIG::Contiguous);
    }

    /// Check if this is a global mapping
    pub fn is_global(&self) -> bool {
        !self.reg.is_set(TTE64_REG::NG)
    }

    /// Set the not-global bit (make it process-specific)
    pub fn set_not_global(&mut self) {
        self.reg.modify(TTE64_REG::NG::NotGlobal);
    }

    /// Check if dirty bit modifier is set (ARMv8.1+)
    pub fn is_dirty_writable(&self) -> bool {
        self.reg.is_set(TTE64_REG::DBM)
    }

    /// Get the software reserved bits
    pub fn sw_reserved(&self) -> u64 {
        self.reg.read(TTE64_REG::SW_RESERVED)
    }

    /// Set the software reserved bits
    pub fn set_sw_reserved(&mut self, value: u64) {
        self.reg.modify(TTE64_REG::SW_RESERVED.val(value & 0xF));
    }
}

// Convenient type aliases for common configurations
/// TTE with 4KB granule and 48-bit output addresses
pub type TTE4K48 = TTE64<Granule4KB, OA48>;

/// TTE with 4KB granule and 52-bit output addresses
pub type TTE4K52 = TTE64<Granule4KB, OA52>;

/// TTE with 16KB granule and 48-bit output addresses
pub type TTE16K48 = TTE64<Granule16KB, OA48>;

/// TTE with 16KB granule and 52-bit output addresses
pub type TTE16K52 = TTE64<Granule16KB, OA52>;

/// TTE with 64KB granule and 48-bit output addresses
pub type TTE64K48 = TTE64<Granule64KB, OA48>;

/// TTE with 64KB granule and 52-bit output addresses
pub type TTE64K52 = TTE64<Granule64KB, OA52>;

/// Constants for different granule sizes block sizes at different levels
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

/// Helper functions for address calculations
impl<G: Granule, O: OA> TTE64<G, O> {
    /// Calculate the index for a virtual address at a given level
    pub fn calculate_index(va: u64, level: usize) -> usize {
        match (G::SHIFT, level) {
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

    /// Check if an address is aligned to the granule boundary
    pub fn is_aligned(addr: u64) -> bool {
        (addr & G::MASK) == 0
    }

    /// Align an address down to the granule boundary
    pub fn align_down(addr: u64) -> u64 {
        addr & !G::MASK
    }

    /// Align an address up to the granule boundary
    pub fn align_up(addr: u64) -> u64 {
        (addr + G::MASK) & !G::MASK
    }
}
