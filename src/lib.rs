//! # aarch64-cpu-ext
//!
//! This crate provides extended functionality and utilities for AArch64 CPU architecture.
//! It builds upon the `aarch64_cpu` crate, offering additional operations for cache management,
//! TLB (Translation Lookaside Buffer) manipulation, and memory management structures.
//!
//! ## Features
//!
//! - **Cache Operations**: High-level and low-level cache management including data cache and instruction cache operations
//! - **TLB Management**: Translation Lookaside Buffer invalidation and management operations
//! - **Memory Structures**: Definitions for Translation Table Entry (TTE) structures and related memory management types
//!
//! ## Platform Support
//!
//! This crate is designed for the AArch64 architecture and will only compile on `aarch64` targets.
//!
//! ## Modules
//!
//! - `asm`: Low-level assembly operations for cache and TLB
//! - `cache`: High-level cache management functions
//! - `structures`: Memory management structures including TTE definitions

#![cfg_attr(not(test), no_std)]

#[cfg(target_arch = "aarch64")]
pub mod asm;
#[cfg(target_arch = "aarch64")]
pub mod cache;
#[cfg(target_arch = "aarch64")]
pub mod registers {
    pub use aarch64_cpu::registers::*;
}

pub mod structures;

#[cfg(test)]
mod test {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
