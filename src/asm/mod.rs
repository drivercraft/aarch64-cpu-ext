//! # Low-level Assembly Operations
//!
//! This module provides low-level assembly operations for AArch64 architecture.
//! It includes wrappers for system instructions related to cache management
//! and TLB (Translation Lookaside Buffer) operations.
//!
//! ## Modules
//!
//! - [`cache`]: Data cache and instruction cache system instructions
//! - [`tlb`]: Translation Lookaside Buffer invalidation instructions
//!
//! ## Design
//!
//! This module uses a type-safe approach to system instructions by defining
//! marker types for each instruction and using traits to provide a unified
//! interface. This prevents invalid instruction/operand combinations at compile time.

pub use aarch64_cpu::asm::*;
pub mod cache;
pub mod tlb;
