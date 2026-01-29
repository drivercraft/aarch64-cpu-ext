//! # Cache System Instructions
//!
//! This module provides type-safe wrappers for AArch64 cache system instructions.
//! It supports both instruction cache (IC) and data cache (DC) operations with
//! compile-time safety through Rust's type system.
//!
//! ## Instruction Cache Operations
//!
//! - `IALLU`: Invalidate all instruction caches to Point of Unification
//! - `IALLUIS`: Invalidate all instruction caches to Point of Unification, Inner Shareable
//!
//! ## Data Cache Operations
//!
//! - `CVAC`: Clean data cache line by VA to PoC
//! - `IVAC`: Invalidate data cache line by VA to PoC
//! - `CIVAC`: Clean and Invalidate data cache line by VA to PoC
//! - `CISW`: Clean and Invalidate data cache line by Set/Way
//! - `ISW`: Invalidate data cache line by Set/Way
//! - `CSW`: Clean data cache line by Set/Way
//!

mod sealed {

    /// Sealed trait for instruction cache operations.
    ///
    /// This trait is not meant to be implemented outside this module.
    /// It provides a type-safe interface for IC (Instruction Cache) instructions.
    pub trait Ic {
        fn ic(&self);
    }

    /// Sealed trait for data cache operations.
    ///
    /// This trait is not meant to be implemented outside this module.
    /// It provides a type-safe interface for DC (Data Cache) instructions.
    pub trait Dc {
        fn dc(&self, addr: u64);
    }
}

macro_rules! ic {
    ($A:ident, $T: ident) => {
        pub struct $T;
        pub const $A: $T = $T {};

        impl sealed::Ic for $T {
            #[inline(always)]
            fn ic(&self) {
                match () {
                    #[cfg(target_arch = "aarch64")]
                    () => unsafe {
                        core::arch::asm!(concat!("ic ", stringify!($A)), options(nostack))
                    },

                    #[cfg(not(target_arch = "aarch64"))]
                    () => unimplemented!(),
                }
            }
        }
    };
}

macro_rules! dc {
    ($A: ident, $T: ident) => {
        pub struct $T;
        pub const $A: $T = $T{};
        impl sealed::Dc for $T {
            #[inline(always)]
            fn dc(&self, addr:u64){
                match() {
                    #[cfg(target_arch = "aarch64")]
                    () => unsafe {
                        core::arch::asm!(concat!("dc ",stringify!($A), ",{}"), in(reg) addr, options(nostack))
                    },
                    #[cfg(not(target_arch = "aarch64"))]
                    () => unimplemented!(),
                }
            }
        }
    }
}

// Generate instruction cache instruction types
ic!(IALLU, Iallu);
ic!(IALLUIS, Ialluis);

// Generate data cache instruction types
dc!(CVAC, Cvac);
dc!(IVAC, Ivac);
dc!(CIVAC, Civac);
dc!(CISW, Cisw);
dc!(ISW, Isw);
dc!(CSW, Csw);

/// Executes an instruction cache operation.
///
/// This function provides a type-safe interface to execute IC (Instruction Cache)
/// instructions. The specific operation is determined by the type of the argument.
///
/// # Arguments
///
/// * `_arg` - The instruction cache operation to perform (e.g., `IALLU`, `IALLUIS`)
///
#[inline(always)]
pub fn ic(_arg: impl sealed::Ic) {
    _arg.ic();
}

/// Executes a data cache operation on a specific address.
///
/// This function provides a type-safe interface to execute DC (Data Cache)
/// instructions with a virtual address operand. The specific operation is
/// determined by the type of the argument.
///
/// # Arguments
///
/// * `_arg` - The data cache operation to perform (e.g., `CVAC`, `IVAC`, `CIVAC`)
/// * `addr` - The virtual address of the cache line to operate on
///
#[inline(always)]
pub fn dc(_arg: impl sealed::Dc, addr: u64) {
    _arg.dc(addr);
}
