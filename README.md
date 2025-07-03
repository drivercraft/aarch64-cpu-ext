# aarch64-cpu-ext

A Rust library providing extended AArch64 CPU utilities and cache management operations.

## Overview

This library extends the functionality of the `aarch64-cpu` crate by providing additional utilities for:

- Cache management operations (invalidate, clean, flush)
- Cache line size detection
- Low-level assembly operations for cache and TLB management
- Register access through re-exported `aarch64-cpu` registers

## Features

- **Cache Operations**: Comprehensive cache management including clean, invalidate, and flush operations
- **Cache Line Size Detection**: Runtime detection of cache line sizes using CTR_EL0 register
- **Assembly Wrappers**: Low-level assembly instruction wrappers for cache and TLB operations
- **No Standard Library**: `#![no_std]` compatible for embedded and bare-metal environments
- **Register Access**: Full access to AArch64 system registers through re-exported functionality

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
aarch64-cpu-ext = "0.1.0"
```

### Cache Operations

```rust
use aarch64_cpu_ext::cache::{icache_flush_all, cache_line_size, dcache_range, CacheOp};

// Flush instruction cache
icache_flush_all();

// Get cache line size
let line_size = cache_line_size();

// Perform cache operations on memory ranges
let buffer = &mut [0u8; 4096];
dcache_range(CacheOp::Clean, buffer.as_ptr() as usize, buffer.len());
dcache_range(CacheOp::Invalidate, buffer.as_ptr() as usize, buffer.len());
dcache_range(CacheOp::CleanAndInvalidate, buffer.as_ptr() as usize, buffer.len());
```

### Register Access

```rust
use aarch64_cpu_ext::registers::*;

// Access system registers (re-exported from aarch64-cpu)
let current_el = CurrentEL.get();
```

### Low-level Assembly Operations

```rust
use aarch64_cpu_ext::asm::cache::{dc, ic, CVAC, IALLU};

// Direct assembly instruction wrappers
ic(IALLU);  // Instruction cache invalidate all
dc(CVAC, address);  // Data cache clean by virtual address
```

## Cache Operation Types

- **Clean**: Write dirty cache lines back to memory without invalidating
- **Invalidate**: Mark cache lines as invalid without writing back
- **CleanAndInvalidate**: Write back dirty lines and mark as invalid

## Requirements

- AArch64 target architecture
- Rust 2024 edition
- No standard library (`#![no_std]`)

## Dependencies

- `aarch64-cpu` version 10 - Provides base AArch64 CPU functionality

## Target Architecture

This library is specifically designed for AArch64 (ARM64) architecture and will not compile for other targets.

## License

This project's license is not specified. Please check with the project maintainers for licensing information.

## Contributing

Contributions are welcome! Please ensure that any new features maintain compatibility with `#![no_std]` environments and follow the existing code patterns.
