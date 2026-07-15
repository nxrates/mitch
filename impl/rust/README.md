# MITCH Rust Reference Implementation

**Official Rust implementation of the MITCH (Moded Individual Trade Clearing and Handling) protocol.**

## Overview

This crate implements the MITCH binary protocol for ultra-low latency market data. It is the reference implementation and executable protocol specification.

- All MITCH message types: Trade, Order, Tick, Index, OrderBook, Bar, Heartbeat + MitchHeader, frames, ticker/channel IDs
- Zero-copy pack/unpack via `#[repr(C, packed)]` structs; optional `bytemuck` Pod/Zeroable for mmap I/O
- Consistent Little-Endian encoding on all platforms
- Constants generated at build time from `../../ids/*.csv` and `../../bins/*.csv` (see `build.rs`)
- Minimal dependencies: `chrono`; `bytemuck`/`serde` optional features
- `cdylib`/`staticlib` output for C-compatible embedding (FFI status: see [../ffi.md](../ffi.md))

## Build & Test

```sh
cargo build --release
cargo test
```

Cross-compiled dynamic libraries (`.so`, `.dylib`, `.dll`) for all supported targets:

```sh
make install-targets       # add Rust targets via rustup
make build-all-platforms   # build every target, artifacts under ../libs/ (gitignored)
```

## Specification

Protocol docs live at the repo root: [model overview](../../model/overview.md), [messaging](../../messaging.md), [framing](../../model/framing.md), [ticker system](../../model/ticker.md).
