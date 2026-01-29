//! # Translation Lookaside Buffer (TLB) Instructions
//!
//! This module provides type-safe wrappers for AArch64 TLB invalidation instructions.
//! TLB invalidation is necessary after modifying page tables or changing translation
//! configurations to ensure the CPU uses the updated translations.
//!
//! ## TLB Instruction Categories
//!
//! ### All TLB Entries
//! - `ALLE1`, `ALLE2`, `ALLE3`: Invalidate all TLB entries for EL1/EL2/EL3
//! - `ALLE1IS`, `ALLE2IS`, `ALLE3IS`: Inner shareable variants
//!
//! ### Virtual Address-based Invalidation
//! - `VAE1`, `VAE2`, `VAE3`: Invalidate by VA and ASID
//! - `VAE1IS`, `VAE2IS`, `VAE3IS`: Inner shareable variants
//! - `VAAE1`: Invalidate by VA only (ASID ignored)
//! - `VAAE1IS`: Inner shareable variant
//!
//! ### ASID-based Invalidation
//! - `ASIDE1`: Invalidate all entries matching a specific ASID
//! - `ASIDE1IS`: Inner shareable variant
//!
//! ### VMID-based Invalidation
//! - `VMALLE1`: Invalidate all stage 1 TLB entries
//! - `VMALLE1IS`: Inner shareable variant
//!

use tock_registers::register_bitfields;

register_bitfields![u64,
    TlbiVA [
        VA OFFSET(0) NUMBITS(44) [],
        TTL OFFSET(44) NUMBITS(4) [],
        ASID OFFSET(48) NUMBITS(16) [],
    ],
    TlbiVAA [
        VA OFFSET(0) NUMBITS(44) [],
        TTL OFFSET(44) NUMBITS(4) [],
    ],
    TlbiRVAA [
        BassADDR OFFSET(0) NUMBITS(37) [],
        TLL  OFFSET(37) NUMBITS(2) [],
        NUM  OFFSET(39) NUMBITS(5) [],
        SCALE OFFSET(44) NUMBITS(2) [],
        TG  OFFSET(46) NUMBITS(2) [],
    ],
    TlbiRVA [
        BassADDR OFFSET(0) NUMBITS(37) [],
        TLL  OFFSET(37) NUMBITS(2) [],
        NUM  OFFSET(39) NUMBITS(5) [],
        SCALE OFFSET(44) NUMBITS(2) [],
        TG  OFFSET(46) NUMBITS(2) [],
        ASID OFFSET(48) NUMBITS(16) [],
    ],
    TlbiASID [
        ASID OFFSET(48) NUMBITS(16) [],
    ],
];

/// Executes a TLB invalidation operation.
///
/// This function provides a type-safe interface to execute TLBI (TLB Invalidate)
/// instructions. The specific operation is determined by the type of the argument.
///
/// # Arguments
///
/// * `val` - The TLB invalidation operation to perform
///
#[inline]
pub fn tlbi(val: impl sealed::Tlbi) {
    val.tlbi();
}

mod sealed {
    /// Sealed trait for TLB invalidation operations.
    ///
    /// This trait is not meant to be implemented outside this module.
    /// It provides a type-safe interface for TLBI (TLB Invalidate) instructions.
    pub trait Tlbi {
        fn tlbi(&self);
    }
}

macro_rules! tlbi_all {
    ($A:ident) => {
        pub struct $A;

        impl sealed::Tlbi for $A {
            #[inline(always)]
            fn tlbi(&self) {
                match () {
                    #[cfg(target_arch = "aarch64")]
                    () => unsafe {
                        core::arch::asm!(concat!("tlbi ", stringify!($A)), options(nostack))
                    },

                    #[cfg(not(target_arch = "aarch64"))]
                    () => unimplemented!(),
                }
            }
        }
    };
}

// Generate all-TLB instruction types for each exception level
tlbi_all!(ALLE1);
tlbi_all!(ALLE2);
tlbi_all!(ALLE3);

// Generate inner shareable variants
tlbi_all!(ALLE1IS);
// tlbi_all!(ALLE1OS);

tlbi_all!(ALLE2IS);
// tlbi_all!(ALLE2OS);

tlbi_all!(ALLE3IS);
// tlbi_all!(ALLE3OS);

// Generate VMID-based all-TLB instruction types
tlbi_all!(VMALLE1);
tlbi_all!(VMALLE1IS);
// tlbi_all!(VMALLE1OS);

/// Converts a virtual address to the TLBI VA format.
///
/// TLBI instructions use VA[55:12] (bits 43:0 in the instruction encoding),
/// so this function shifts the VA right by 12 bits and masks to 44 bits.
///
/// # Arguments
///
/// * `va` - The virtual address to convert
///
/// # Returns
///
/// The VA in TLBI instruction format (VA[55:12])
#[inline]
fn va_to_tlbi_va(va: usize) -> u64 {
    const VA_MASK: u64 = (1 << 44) - 1; // VA[55:12] => bits[43:0]
    (va as u64 >> 12) & VA_MASK
}

/// Macro to generate VA-based TLB invalidation instruction types.
///
/// This macro creates structures for TLB invalidation operations that take
/// a virtual address and ASID (Address Space Identifier) as parameters.
macro_rules! tlbi_va {
    ($A:ident) => {
        pub struct $A(u64);

        impl $A {
            /// Creates a new VA-based TLB invalidation operation.
            ///
            /// # Arguments
            ///
            /// * `asid` - The Address Space Identifier (ASID)
            /// * `va` - The virtual address to invalidate
            ///
            /// # Returns
            ///
            /// A new instance of the TLB invalidation operation
            #[inline]
            pub fn new(asid: usize, va: usize) -> Self {
                Self((TlbiVA::VA.val(va_to_tlbi_va(va)) +
                    TlbiVA::ASID.val(asid as u64)).value)
            }
        }

        impl sealed::Tlbi for $A {
            #[inline(always)]
            fn tlbi(&self) {
                match () {
                    #[cfg(target_arch = "aarch64")]
                    () => unsafe {
                        core::arch::asm!(concat!("tlbi ", stringify!($A), ", {}"), in(reg) self.0, options(nostack))
                    },

                    #[cfg(not(target_arch = "aarch64"))]
                    () => unimplemented!(),
                }
            }
        }
    };
}

tlbi_va!(VAE1);
tlbi_va!(VAE2);
tlbi_va!(VAE3);

tlbi_va!(VAE1IS);
// tlbi_va!(VAE1OS);

tlbi_va!(VAE2IS);
// tlbi_va!(VAE2OS);

tlbi_va!(VAE3IS);
// tlbi_va!(VAE3OS);

/// Macro to generate ASID-based TLB invalidation instruction types.
///
/// This macro creates structures for TLB invalidation operations that invalidate
/// all entries matching a specific ASID (Address Space Identifier).
macro_rules! tlbi_asid {
    ($A:ident) => {
        pub struct $A(u64);

        impl $A {
            /// Creates a new ASID-based TLB invalidation operation.
            ///
            /// This invalidates all TLB entries that match the specified ASID,
            /// regardless of the virtual address.
            ///
            /// # Arguments
            ///
            /// * `asid` - The Address Space Identifier (ASID)
            ///
            /// # Returns
            ///
            /// A new instance of the TLB invalidation operation
            #[inline]
            pub fn new(asid: usize) -> Self {
                Self(TlbiASID::ASID.val(asid as u64).value)
            }
        }

        impl sealed::Tlbi for $A {
            #[inline(always)]
            fn tlbi(&self) {
                match () {
                    #[cfg(target_arch = "aarch64")]
                    () => unsafe {
                        core::arch::asm!(concat!("tlbi ", stringify!($A), ", {}"), in(reg) self.0, options(nostack))
                    },

                    #[cfg(not(target_arch = "aarch64"))]
                    () => unimplemented!(),
                }
            }
        }
    };
}

tlbi_asid!(ASIDE1);
tlbi_asid!(ASIDE1IS);
// tlbi_asid!(ASIDE1OS);

/// Macro to generate VAA-based TLB invalidation instruction types.
///
/// This macro creates structures for TLB invalidation operations that take
/// only a virtual address (ignoring the ASID) as a parameter.
macro_rules! tlbi_vaa {
    ($A:ident) => {
        pub struct $A(u64);

        impl $A {
            /// Creates a new VAA-based TLB invalidation operation.
            ///
            /// Unlike VA-based operations, VAA ignores the ASID and invalidates
            /// entries across all address spaces matching the virtual address.
            ///
            /// # Arguments
            ///
            /// * `va` - The virtual address to invalidate
            ///
            /// # Returns
            ///
            /// A new instance of the TLB invalidation operation
            #[inline]
            pub fn new(va: usize) -> Self {
                Self(TlbiVAA::VA.val(va_to_tlbi_va(va)).value)
            }
        }

        impl sealed::Tlbi for $A {
            #[inline(always)]
            fn tlbi(&self) {
                match () {
                    #[cfg(target_arch = "aarch64")]
                    () => unsafe {
                        core::arch::asm!(concat!("tlbi ", stringify!($A), ", {}"), in(reg) self.0, options(nostack))
                    },

                    #[cfg(not(target_arch = "aarch64"))]
                    () => unimplemented!(),
                }
            }
        }
    };
}

tlbi_vaa!(VAAE1);
tlbi_vaa!(VAAE1IS);
// tlbi_vaa!(VAAE1OS);
