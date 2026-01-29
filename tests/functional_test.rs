//! Functional Tests
//!
//! This module contains functional tests that verify component behavior through
//! simulated environments or by testing pure logic without requiring actual hardware.

use aarch64_cpu_ext::structures::tte::{
    AccessPermission, Granule, Granule4KB, Granule16KB, Granule64KB, OA, OA48, OA52, TTE64,
};

// ============================================================================
// Granule Type Tests
// ============================================================================

#[test]
fn test_granule_4kb_properties() {
    assert_eq!(Granule4KB::M, 12);
    assert_eq!(Granule4KB::SIZE, 4096);
    assert_eq!(Granule4KB::MASK, 0xFFF);
}

#[test]
fn test_granule_16kb_properties() {
    assert_eq!(Granule16KB::M, 14);
    assert_eq!(Granule16KB::SIZE, 16384);
    assert_eq!(Granule16KB::MASK, 0x3FFF);
}

#[test]
fn test_granule_64kb_properties() {
    assert_eq!(Granule64KB::M, 16);
    assert_eq!(Granule64KB::SIZE, 65536);
    assert_eq!(Granule64KB::MASK, 0xFFFF);
}

#[test]
fn test_granule_alignment() {
    // Test that addresses are properly aligned to granule boundaries
    assert_eq!(0 & Granule4KB::MASK, 0, "Zero address should be aligned");
    assert_eq!(4096 & Granule4KB::MASK, 0, "4096 should be 4KB-aligned");
    assert_eq!(8192 & Granule4KB::MASK, 0, "8192 should be 4KB-aligned");
    assert_ne!(100 & Granule4KB::MASK, 0, "100 should not be 4KB-aligned");
}

// ============================================================================
// Output Address (OA) Type Tests
// ============================================================================

#[test]
fn test_oa48_properties() {
    assert_eq!(OA48::BITS, 48);
    assert_eq!(
        1u64 << OA48::BITS,
        281474976710656,
        "48-bit address space = 256 TB"
    );
}

#[test]
fn test_oa52_properties() {
    assert_eq!(OA52::BITS, 52);
    assert_eq!(
        1u64 << OA52::BITS,
        4503599627370496,
        "52-bit address space = 4 PB"
    );
}

// ============================================================================
// AccessPermission Tests
// ============================================================================

#[test]
fn test_access_permission_conversion() {
    // Test bits -> AccessPermission
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
}

#[test]
fn test_access_permission_invalid_bits() {
    // Bits outside the 2-bit field should be masked
    assert_eq!(
        AccessPermission::from_bits(0b100),
        Some(AccessPermission::PrivilegedReadWrite),
        "Only lower 2 bits should be considered"
    );
    assert_eq!(
        AccessPermission::from_bits(0b101),
        Some(AccessPermission::ReadWrite)
    );
    assert_eq!(
        AccessPermission::from_bits(0xFF),
        Some(AccessPermission::ReadOnly)
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
fn test_access_permission_unprivileged_access() {
    assert!(AccessPermission::ReadWrite.allows_unprivileged());
    assert!(AccessPermission::ReadOnly.allows_unprivileged());
    assert!(!AccessPermission::PrivilegedReadWrite.allows_unprivileged());
    assert!(!AccessPermission::PrivilegedReadOnly.allows_unprivileged());
}

#[test]
fn test_access_permission_privileged_write() {
    assert!(AccessPermission::PrivilegedReadWrite.allows_privileged_write());
    assert!(AccessPermission::ReadWrite.allows_privileged_write());
    assert!(!AccessPermission::PrivilegedReadOnly.allows_privileged_write());
    assert!(!AccessPermission::ReadOnly.allows_privileged_write());
}

#[test]
fn test_access_permission_unprivileged_write() {
    assert!(AccessPermission::ReadWrite.allows_unprivileged_write());
    assert!(!AccessPermission::PrivilegedReadWrite.allows_unprivileged_write());
    assert!(!AccessPermission::PrivilegedReadOnly.allows_unprivileged_write());
    assert!(!AccessPermission::ReadOnly.allows_unprivileged_write());
}

// ============================================================================
// TTE64 Creation and Basic Operations Tests
// ============================================================================

#[test]
fn test_tte64_new_from_raw() {
    let tte = TTE64::<Granule4KB, OA48>::new(0x123456789ABCDEF0);
    assert_eq!(tte.get(), 0x123456789ABCDEF0);
}

#[test]
fn test_tte64_invalid() {
    let tte = TTE64::<Granule4KB, OA48>::invalid();
    assert_eq!(tte.get(), 0);
    assert!(!tte.is_valid());
}

#[test]
fn test_tte64_is_valid() {
    let mut tte = TTE64::<Granule4KB, OA48>::new(0);
    assert!(!tte.is_valid());

    tte.set_is_valid(true);
    assert!(tte.is_valid());

    tte.set_is_valid(false);
    assert!(!tte.is_valid());
}

// ============================================================================
// TTE64 Table Entry Tests
// ============================================================================

#[test]
fn test_tte64_new_table_aligned() {
    // Create a table entry with an aligned address
    let tte = TTE64::<Granule4KB, OA48>::new_table(0x1000);
    assert!(tte.is_valid());
    assert!(tte.is_table());
    assert!(!tte.is_block());
}

#[test]
#[should_panic(expected = "Address must be aligned to granule size")]
fn test_tte64_new_table_unaligned() {
    // This should panic because the address is not 4KB aligned
    TTE64::<Granule4KB, OA48>::new_table(0x1001);
}

#[test]
#[should_panic(expected = "Address exceeds output address width")]
fn test_tte64_new_table_address_overflow() {
    // This should panic because the address exceeds 48-bit OA
    TTE64::<Granule4KB, OA48>::new_table(1u64 << 48);
}

#[test]
fn test_tte64_set_is_table() {
    let mut tte = TTE64::<Granule4KB, OA48>::new(0);
    tte.set_is_valid(true);
    tte.set_is_table();
    assert!(tte.is_table());
    assert!(!tte.is_block());
}

// ============================================================================
// TTE64 Block Entry Tests
// ============================================================================

#[test]
fn test_tte64_new_block_aligned() {
    // Create a block entry with an aligned address
    let tte = TTE64::<Granule4KB, OA48>::new_block(0x2000);
    assert!(tte.is_valid());
    assert!(tte.is_block());
    assert!(!tte.is_table());
}

#[test]
#[should_panic(expected = "Address must be aligned to granule size")]
fn test_tte64_new_block_unaligned() {
    // This should panic because the address is not 4KB aligned
    TTE64::<Granule4KB, OA48>::new_block(0x2001);
}

#[test]
fn test_tte64_set_is_block() {
    let mut tte = TTE64::<Granule4KB, OA48>::new(0);
    tte.set_is_valid(true);
    tte.set_is_block();
    assert!(tte.is_block());
    assert!(!tte.is_table());
}

// ============================================================================
// TTE64 Address Tests
// ============================================================================

#[test]
fn test_tte64_set_address_4kb() {
    let mut tte = TTE64::<Granule4KB, OA48>::new(0);
    tte.set_is_valid(true);
    tte.set_is_table();

    let addr = 0x1000u64;
    tte.set_address(addr);
    // The address stored should have the lower 12 bits masked
    assert_eq!(tte.address(), addr);
}

#[test]
fn test_tte64_set_address_16kb() {
    let mut tte = TTE64::<Granule16KB, OA48>::new(0);
    tte.set_is_valid(true);
    tte.set_is_table();

    let addr = 0x4000u64;
    tte.set_address(addr);
    assert_eq!(tte.address(), addr);
}

#[test]
fn test_tte64_set_address_64kb() {
    let mut tte = TTE64::<Granule64KB, OA48>::new(0);
    tte.set_is_valid(true);
    tte.set_is_table();

    let addr = 0x10000u64;
    tte.set_address(addr);
    assert_eq!(tte.address(), addr);
}

#[test]
fn test_tte64_address_zero() {
    let mut tte = TTE64::<Granule4KB, OA48>::new(0);
    tte.set_is_valid(true);
    tte.set_is_table();
    tte.set_address(0);
    assert_eq!(tte.address(), 0);
}

#[test]
fn test_tte64_address_max_valid_48bit() {
    let mut tte = TTE64::<Granule4KB, OA48>::new(0);
    tte.set_is_valid(true);
    tte.set_is_table();

    // Maximum 48-bit aligned address
    let addr = (1u64 << 48) - 4096;
    tte.set_address(addr);
    assert_eq!(tte.address(), addr);
}

#[test]
fn test_tte64_address_with_52bit_oa() {
    let mut tte = TTE64::<Granule4KB, OA52>::new(0);
    tte.set_is_valid(true);
    tte.set_is_table();

    let addr = 0x1000u64;
    tte.set_address(addr);
    assert_eq!(tte.address(), addr);
}

#[test]
fn test_tte64_invalid_entry_address() {
    // Invalid entry should return 0 for address
    let tte = TTE64::<Granule4KB, OA48>::invalid();
    assert_eq!(tte.address(), 0);
}

// ============================================================================
// TTE64 Edge Cases and Boundary Tests
// ============================================================================

#[test]
fn test_tte64_multiple_operations() {
    // Test a sequence of operations on a single TTE
    let mut tte = TTE64::<Granule4KB, OA48>::new(0);

    // Start as table
    tte.set_is_valid(true);
    tte.set_is_table();
    tte.set_address(0x1000);
    assert!(tte.is_table());
    assert_eq!(tte.address(), 0x1000);

    // Switch to block
    tte.set_is_block();
    assert!(tte.is_block());
    assert!(!tte.is_table());
    // Address should be preserved
    assert_eq!(tte.address(), 0x1000);

    // Change address
    tte.set_address(0x2000);
    assert_eq!(tte.address(), 0x2000);
}

#[test]
fn test_tte64_raw_value_preservation() {
    // Create a TTE with specific raw value and verify round-trip
    let raw_value = 0x8000000000051F;
    let tte = TTE64::<Granule4KB, OA48>::new(raw_value);
    assert_eq!(tte.get(), raw_value);
}

#[test]
fn test_tte64_with_all_granules() {
    // Test the same address with different granule types
    let addr = 0x10000; // 64KB aligned, also 16KB and 4KB aligned

    let tte_4k = TTE64::<Granule4KB, OA48>::new_table(addr);
    assert!(tte_4k.is_valid());
    assert_eq!(tte_4k.address(), addr);

    let tte_16k = TTE64::<Granule16KB, OA48>::new_table(addr);
    assert!(tte_16k.is_valid());
    assert_eq!(tte_16k.address(), addr);

    let tte_64k = TTE64::<Granule64KB, OA48>::new_table(addr);
    assert!(tte_64k.is_valid());
    assert_eq!(tte_64k.address(), addr);
}
