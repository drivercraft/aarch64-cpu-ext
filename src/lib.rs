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
mod tests {
    use crate::structures::tte::{
        AccessPermission, Granule, Granule16KB, Granule4KB, Granule64KB, OA, OA48, OA52, TTE64,
    };

    #[test]
    fn test_basic_arithmetic() {
        // Basic sanity test for the test framework
        assert_eq!(2 + 2, 4);
        assert_eq!(10 - 5, 5);
        assert_eq!(3 * 3, 9);
    }

    #[test]
    fn test_boolean_logic() {
        // Basic boolean logic tests
        assert!(true);
        assert!(!false);
        assert_eq!(true && true, true);
        assert_eq!(true || false, true);
    }

    #[test]
    fn test_granule_size_properties() {
        // Test granule size properties
        assert_eq!(Granule4KB::M, 12);
        assert_eq!(Granule4KB::SIZE, 4096);
        assert_eq!(Granule4KB::MASK, 0xFFF);

        assert_eq!(Granule16KB::M, 14);
        assert_eq!(Granule16KB::SIZE, 16384);
        assert_eq!(Granule16KB::MASK, 0x3FFF);

        assert_eq!(Granule64KB::M, 16);
        assert_eq!(Granule64KB::SIZE, 65536);
        assert_eq!(Granule64KB::MASK, 0xFFFF);
    }

    #[test]
    fn test_access_permission_from_bits() {
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
        // Note: from_bits uses bits & 0b11, so 0b100 becomes 0b00
        assert_eq!(
            AccessPermission::from_bits(0b100),
            Some(AccessPermission::PrivilegedReadWrite)
        );
    }

    #[test]
    fn test_access_permission_as_bits() {
        assert_eq!(AccessPermission::PrivilegedReadWrite.as_bits(), 0b00);
        assert_eq!(AccessPermission::ReadWrite.as_bits(), 0b01);
        assert_eq!(AccessPermission::PrivilegedReadOnly.as_bits(), 0b10);
        assert_eq!(AccessPermission::ReadOnly.as_bits(), 0b11);
    }

    #[test]
    fn test_access_permission_checks() {
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
    fn test_access_permission_equality() {
        assert_eq!(AccessPermission::ReadWrite, AccessPermission::ReadWrite);
        assert_ne!(AccessPermission::ReadWrite, AccessPermission::ReadOnly);
        assert_eq!(AccessPermission::PrivilegedReadOnly, AccessPermission::PrivilegedReadOnly);
    }

    #[test]
    fn test_oa_bits() {
        assert_eq!(OA48::BITS, 48);
        assert_eq!(OA52::BITS, 52);
    }

    #[test]
    fn test_tte_invalid() {
        let tte = TTE64::<Granule4KB, OA48>::invalid();
        assert!(!tte.is_valid());
        assert!(!tte.is_table());
        assert!(!tte.is_block());
        assert_eq!(tte.get(), 0);
    }

    #[test]
    fn test_tte_new() {
        let tte = TTE64::<Granule4KB, OA48>::new(0x1234_5678);
        assert_eq!(tte.get(), 0x1234_5678);
    }

    #[test]
    fn test_tte_alignment_helpers_4kb() {
        type TTE = TTE64<Granule4KB, OA48>;

        // Test is_aligned
        assert!(TTE::is_aligned(0x1000));
        assert!(TTE::is_aligned(0x2000));
        assert!(!TTE::is_aligned(0x1001));
        assert!(!TTE::is_aligned(0xFFF));

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

    #[test]
    fn test_tte_alignment_helpers_64kb() {
        type TTE = TTE64<Granule64KB, OA48>;

        // Test is_aligned
        assert!(TTE::is_aligned(0x1_0000));
        assert!(TTE::is_aligned(0x2_0000));
        assert!(!TTE::is_aligned(0x1_0001));
        assert!(!TTE::is_aligned(0xFFFF));

        // Test align_down
        assert_eq!(TTE::align_down(0x1_FFFF), 0x1_0000);
        assert_eq!(TTE::align_down(0x2_0000), 0x2_0000);

        // Test align_up
        assert_eq!(TTE::align_up(0x1_0000), 0x1_0000);
        assert_eq!(TTE::align_up(0x1_0001), 0x2_0000);
        assert_eq!(TTE::align_up(0x1_FFFF), 0x2_0000);
    }

    #[test]
    fn test_tte_set_is_valid() {
        let mut tte = TTE64::<Granule4KB, OA48>::invalid();
        assert!(!tte.is_valid());

        tte.set_is_valid(true);
        assert!(tte.is_valid());

        tte.set_is_valid(false);
        assert!(!tte.is_valid());
    }

    #[test]
    fn test_tte_set_is_table_block() {
        let mut tte = TTE64::<Granule4KB, OA48>::new(0);
        tte.set_is_valid(true);

        tte.set_is_table();
        assert!(tte.is_table());
        assert!(!tte.is_block());

        tte.set_is_block();
        assert!(!tte.is_table());
        assert!(tte.is_block());
    }

    #[test]
    fn test_tte_attr_index() {
        let mut tte = TTE64::<Granule4KB, OA48>::new(0);

        for i in 0..8 {
            tte.set_attr_index(i);
            assert_eq!(tte.attr_index(), i);
        }
    }

    #[test]
    fn test_tte_executable() {
        let mut tte = TTE64::<Granule4KB, OA48>::new(0);

        tte.set_executable(true);
        assert!(tte.is_executable());

        tte.set_executable(false);
        assert!(!tte.is_executable());
    }

    #[test]
    fn test_tte_privileged_executable() {
        let mut tte = TTE64::<Granule4KB, OA48>::new(0);

        tte.set_privileged_executable(true);
        assert!(tte.is_privileged_executable());

        tte.set_privileged_executable(false);
        assert!(!tte.is_privileged_executable());
    }

    #[test]
    fn test_tte_access_permission() {
        let mut tte = TTE64::<Granule4KB, OA48>::new(0);

        for &perm in &[
            AccessPermission::PrivilegedReadWrite,
            AccessPermission::ReadWrite,
            AccessPermission::PrivilegedReadOnly,
            AccessPermission::ReadOnly,
        ] {
            tte.set_access_permission(perm);
            assert_eq!(tte.access_permission(), perm);
        }
    }

    #[test]
    fn test_tte_shareability() {
        let mut tte = TTE64::<Granule4KB, OA48>::new(0);

        tte.set_shareability(crate::structures::tte::Shareability::NonShareable);
        assert_eq!(
            tte.shareability(),
            crate::structures::tte::Shareability::NonShareable
        );

        tte.set_shareability(crate::structures::tte::Shareability::InnerShareable);
        assert_eq!(
            tte.shareability(),
            crate::structures::tte::Shareability::InnerShareable
        );

        tte.set_shareability(crate::structures::tte::Shareability::OuterShareable);
        assert_eq!(
            tte.shareability(),
            crate::structures::tte::Shareability::OuterShareable
        );
    }

    #[test]
    fn test_tte_access_flag() {
        let mut tte = TTE64::<Granule4KB, OA48>::new(0);

        assert!(!tte.is_accessed());

        tte.set_access();
        assert!(tte.is_accessed());

        tte.clear_access();
        assert!(!tte.is_accessed());
    }

    #[test]
    fn test_tte_contiguous() {
        let mut tte = TTE64::<Granule4KB, OA48>::new(0);

        assert!(!tte.is_contiguous());

        tte.set_contiguous();
        assert!(tte.is_contiguous());
    }

    #[test]
    fn test_tte_global() {
        let mut tte = TTE64::<Granule4KB, OA48>::new(0);

        // Default is global
        assert!(tte.is_global());

        tte.set_not_global();
        assert!(!tte.is_global());
    }

    #[test]
    fn test_tte_sw_reserved() {
        let mut tte = TTE64::<Granule4KB, OA48>::new(0);

        for value in 0..16 {
            tte.set_sw_reserved(value);
            assert_eq!(tte.sw_reserved(), value);
        }

        // Test that values above 15 are masked
        tte.set_sw_reserved(0xFF);
        assert_eq!(tte.sw_reserved(), 0xF);
    }

    #[test]
    fn test_block_sizes_constants() {
        use crate::structures::tte::block_sizes;

        // 4KB granule block sizes
        assert_eq!(block_sizes::granule_4k::LEVEL1_BLOCK_SIZE, 1024 * 1024 * 1024);
        assert_eq!(block_sizes::granule_4k::LEVEL2_BLOCK_SIZE, 2 * 1024 * 1024);
        assert_eq!(block_sizes::granule_4k::LEVEL3_PAGE_SIZE, 4 * 1024);

        // 16KB granule block sizes
        assert_eq!(block_sizes::granule_16k::LEVEL1_BLOCK_SIZE, 64 * 1024 * 1024 * 1024);
        assert_eq!(block_sizes::granule_16k::LEVEL2_BLOCK_SIZE, 32 * 1024 * 1024);
        assert_eq!(block_sizes::granule_16k::LEVEL3_PAGE_SIZE, 16 * 1024);

        // 64KB granule block sizes
        assert_eq!(block_sizes::granule_64k::LEVEL1_BLOCK_SIZE, 4 * 1024 * 1024 * 1024);
        assert_eq!(block_sizes::granule_64k::LEVEL2_BLOCK_SIZE, 512 * 1024 * 1024);
        assert_eq!(block_sizes::granule_64k::LEVEL3_PAGE_SIZE, 64 * 1024);
    }
}
