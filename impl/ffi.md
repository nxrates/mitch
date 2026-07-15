# MITCH FFI & Language Bindings

## Status

The Rust crate (`rust/`) is the reference implementation and compiles as `cdylib`/`staticlib` (see `rust/Cargo.toml`), but **currently exports no C ABI functions**: the previous FFI layer and the symbol-resolution logic were moved out of this crate (see `rust/src/lib.rs`). The `ffi` cargo feature and the design below describe the intended interface for when bindings are (re)built.

Standalone ports (no FFI required): `typescript/mitch.ts`, `python/`, plus `c/`, `cpp/`, `csharp/`, `go/`, `java/`, `zig/`.

## Design Principles

1. **C-compatible interface**: `extern "C"` functions, primitive C types only, error handling via return codes and out parameters, explicit alloc/free.
2. **Zero-copy**: wrap the existing `pack()`/`unpack()` methods; buffer-based APIs for batches. C structures mirror the Rust `#[repr(C, packed)]` layouts byte-for-byte.
3. **Thread safety**: no shared mutable state across the boundary; each binding language handles concurrency at its level.

## Planned Surface

One `mitch_pack_<type>()` / `mitch_unpack_<type>()` pair per message type (Trade, Order, Tick, Index, Bar, OrderBook, Heartbeat, MitchHeader), wrapping the corresponding Rust `pack`/`unpack`; plus:

- `mitch_create_ticker_id()` / `mitch_decode_ticker_id()` wrapping `TickerId`
- `mitch_create_channel()` wrapping `ChannelId::new()`
- `mitch_get_message_sizes()` returning the `message_sizes` constants

Symbol/asset resolution is out of scope for the FFI layer (it lives in consumer SDKs; see [../model/ticker.md](../model/ticker.md#part-3-symbol-resolution)).

## Build Targets

Cross-compilation is automated via `rust/Makefile`:

1. `make install-targets` adds all required Rust targets via `rustup`
2. `make build-all-platforms` compiles for every target (outputs organized under `../libs/`, gitignored)

| Platform | Targets | Tier |
|----------|---------|------|
| Linux | `x86_64-unknown-linux-gnu` | 1 |
| | `i686-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` | 2 |
| macOS | `x86_64-apple-darwin`, `aarch64-apple-darwin` | 1 |
| Windows | `x86_64-pc-windows-msvc`, `i686-pc-windows-msvc`, `x86_64-pc-windows-gnu` | 1 |

Release builds use LTO, `codegen-units = 1`, symbol stripping, and `panic = "abort"` (required for FFI safety).

## Binding Requirements

- Byte-identical wire output vs the Rust reference (round-trip serialization tests per type, mirroring `rust/tests/`)
- <100ns FFI overhead for pack/unpack vs native Rust
- Buffer length validation at the boundary; no undefined behavior on malformed input
