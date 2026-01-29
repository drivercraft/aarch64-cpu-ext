//! # Data Structures for AArch64 Memory Management
//!
//! This module contains data structures and types used for AArch64 memory management,
//! particularly for managing translation tables and page table entries.
//!
//! ## Modules
//!
//! - [`tte`]: Translation Table Entry (TTE) structures and related types
//!
//! ## Overview
//!
//! AArch64 uses multi-level page tables to translate virtual addresses to physical
//! addresses. The structures in this module provide type-safe abstractions for
//! creating and manipulating these translation table entries.

pub mod tte;
