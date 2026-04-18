# MITCH FFI & Language Bindings Specifications

## Overview

The MITCH Rust reference implementation (see `rust/src/lib.rs`) serves as the foundation for cross-platform language bindings. This document specifies the Foreign Function Interface (FFI) design and the Python wrapper implementation (`mitch-py`).

## Target Languages & Priority

### Binding Targets
- **Python** (Primary implementation target)
- **Node.js/Deno/Bun** (JavaScript/TypeScript ecosystem)
- **Java** (JNI bindings)
- **C++** (Native integration)
- **C#** (P/Invoke/.NET)
- **Go** (CGO bindings)

### Implementation Focus
This specification prioritizes **Python bindings** as the initial implementation, with architecture designed to support all target languages.

## FFI Architecture Principles

### 1. C-Compatible Interface
- All FFI functions use C calling conventions (`extern "C"`)
- Only primitive C types in function signatures (no Rust-specific types)
- Consistent error handling via return codes and out parameters
- Memory management through explicit allocation/deallocation functions

### 2. Zero-Copy Design
- Leverage existing zero-copy message packing from `src/trade.rs`, `src/order.rs`, etc.
- Direct memory access for performance-critical operations
- Buffer-based APIs for batch processing

### 3. Thread Safety
- All FFI functions must be thread-safe
- No shared mutable state in the C interface
- Each binding language handles concurrency at its level

## FFI Implementation Status

**Current State:** The Rust library contains complete implementations of all MITCH protocol features, but FFI exports are minimal (see `src/lib.rs` TODO comment). The FFI layer needs to be implemented to expose existing functionality.

**Implementation Strategy:** Add `#[no_mangle] extern "C"` functions to `src/lib.rs` that wrap existing Rust APIs with C-compatible signatures.

## Core FFI Interface Components

### 1. Message Type Operations

**Reference Implementation:** `src/trade.rs`, `src/order.rs`, `src/tick.rs`, `src/index.rs`, `src/order_book.rs`

All message types already implement `pack()` and `unpack()` methods with zero-copy operations.

#### Required FFI Functions (To Be Implemented)
- `mitch_pack_trade()`, `mitch_unpack_trade()` → wraps `Trade::pack()`, `Trade::unpack()`
- `mitch_pack_order()`, `mitch_unpack_order()` → wraps `Order::pack()`, `Order::unpack()`  
- `mitch_pack_tick()`, `mitch_unpack_tick()` → wraps `Tick::pack()`, `Tick::unpack()`
- `mitch_pack_index()`, `mitch_unpack_index()` → wraps `Index::pack()`, `Index::unpack()`
- `mitch_pack_order_book()`, `mitch_unpack_order_book()` → wraps `OrderBook::pack()`, `OrderBook::unpack()`
- `mitch_get_message_sizes()` → returns message size constants

### 2. Asset Resolution System

**Reference Implementation:** `src/ticker.rs` functions: `resolve_asset()`, `get_asset_by_id()`, etc.

The Rust library already implements comprehensive asset resolution with fuzzy matching via `AssetResolver`.

#### Required FFI Functions (To Be Implemented)
- `mitch_resolve_asset()` → wraps `resolve_asset()`
- `mitch_resolve_asset_in_class()` → wraps `resolve_asset_in_class()`
- `mitch_get_asset_by_id()` → wraps `get_asset_by_id()`
- `mitch_pack_asset()`, `mitch_unpack_asset()` → wraps `pack_asset()`, `unpack_asset()`

### 3. Ticker Resolution System

**Reference Implementation:** `src/ticker.rs` function: `resolve_ticker()`

Complete ticker resolution with automatic quote detection and suffix stripping is already implemented.

#### Required FFI Functions (To Be Implemented)
- `mitch_resolve_ticker()` → wraps `resolve_ticker()`
- `mitch_create_ticker_id()` → wraps `TickerId::new()`
- `mitch_decode_ticker_id()` → wraps `TickerId` field extraction methods

### 4. Market Provider Resolution

**Reference Implementation:** `src/market_providers.rs` functions: `find_market_provider()`, `get_market_provider_by_id()`, etc.

Market provider resolution with exact matching is implemented; fuzzy matching infrastructure exists.

#### Required FFI Functions (To Be Implemented)
- `mitch_find_market_provider()` → wraps `find_market_provider()`
- `mitch_get_market_provider_by_id()` → wraps `get_market_provider_by_id()`
- `mitch_get_all_market_providers()` → wraps `get_all_market_providers()`

### 5. Channel Resolution System

**Reference Implementation:** `src/channel.rs` - `ChannelId` struct with `new()`, `pack()`, `unpack()`

Channel ID creation and manipulation is fully implemented.

#### Required FFI Functions (To Be Implemented)
- `mitch_create_channel()` → wraps `ChannelId::new()`
- `mitch_pack_channel()`, `mitch_unpack_channel()` → wraps `ChannelId::pack()`, `ChannelId::unpack()`

### 6. Header Operations

**Reference Implementation:** `src/header.rs` - `MitchHeader` with complete implementation

Header operations are fully implemented with validation and 48-bit timestamp handling.

#### Required FFI Functions (To Be Implemented)
- `mitch_create_header()` → wraps `MitchHeader::new()`
- `mitch_pack_header()`, `mitch_unpack_header()` → wraps `MitchHeader::pack()`, `MitchHeader::unpack()`

## Python Wrapper Architecture (`python/`)

### 1. Simplified Package Structure
```
python/
├── mitch/
│   ├── __init__.py          # Main API exports & library loading
│   ├── types.py             # Python dataclasses (Asset, Ticker, etc.)
│   ├── _ffi.py              # ctypes/cffi FFI bindings
│   └── exceptions.py        # Exception mapping from Rust errors
├── tests/                   # Test suite (mirrors Rust tests)
└── setup.py                # Package configuration
```

**Rationale:** Since all resolution logic exists in Rust, Python wrapper is minimal - just FFI bindings + convenience types.

### 2. FFI Binding Strategy

#### Library Loading (`__init__.py`)
- Load compiled Rust library (`.so`/`.dylib`/`.dll`) 
- Platform detection and library path resolution
- Version compatibility checking via `mitch_get_version()`

#### ctypes Bindings (`_ffi.py`)
- Direct ctypes function signatures for all `mitch_*` FFI functions
- C structure definitions matching Rust `#[repr(C, packed)]`
- Buffer management for pack/unpack operations
- Error code handling and exception raising

### 3. Python Type Layer (`types.py`)

#### Minimal Dataclasses
- **Asset**: `id: int, class_id: int, class: AssetClass, name: str, aliases: str`
- **Ticker**: `id: int, name: str, instrument_type: InstrumentType, base: Asset, quote: Asset`
- **MarketProvider**: `id: int, name: str`
- **Channel**: `id: int, provider: int, msg_type: str`
- **Message Types**: Trade, Order, Tick, Index, OrderBook with field validation

#### Conversion Methods
- `from_ffi()` - Create Python object from FFI result
- `to_ffi()` - Convert Python object to FFI parameters (where needed)
- Auto-conversion in wrapper functions

### 4. API Design Principles

#### Direct FFI Wrapping
- Each `mitch_*` FFI function gets a Python wrapper
- Minimal Python-side logic - just parameter validation and type conversion
- Error handling via exception mapping
- No reimplementation of resolution algorithms

#### Performance Priority
- Zero-copy where possible (direct buffer access)
- Minimal Python object creation overhead
- Optional numpy array support for bulk operations
- Leverage Rust's performance for all heavy operations

## Testing Requirements

### 1. Test Coverage Parity
The Python test suite must achieve **100% parity** with the Rust test suite coverage:

**Reference Tests:** `rust/tests/`
- `header_test.rs` → Python header tests
- `trade_test.rs` → Python trade message tests
- `order_test.rs` → Python order message tests
- `tick_test.rs` → Python tick message tests
- `index_test.rs` → Python index message tests
- `order_book_test.rs` → Python order book tests
- `ticker_test.rs` → Python ticker resolution tests
- `channel_test.rs` → Python channel tests

### 2. Additional Python-Specific Tests
- FFI library loading across platforms
- ctypes parameter passing and return value handling
- Python type conversion accuracy
- Memory safety (no segfaults from invalid FFI calls)
- Performance overhead measurement vs native Rust

### 3. Cross-Platform Testing
- Linux (primary development platform)
- macOS (Darwin support) 
- Windows (via CI/CD)
- Python versions 3.8+

## Build & Distribution

### 1. Rust FFI Library Build
- Compile to `.so` (Linux), `.dylib` (macOS), `.dll` (Windows)
- Static linking of dependencies to minimize deployment complexity
- Optimized release builds with LTO enabled
- Debug symbol stripping for production builds

### 2. Cross-Compilation Strategy

All selected build targets are safe, well-established, and officially supported by the Rust toolchain, ensuring reliable cross-platform builds.

| Platform | Supported Targets | Status | Notes |
|----------|-------------------|--------|-------|
| **Linux** | `x86_64-unknown-linux-gnu` | Tier 1 | Most common 64-bit Linux |
| | `i686-unknown-linux-gnu` | Tier 2 | Legacy 32-bit Linux |
| | `aarch64-unknown-linux-gnu` | Tier 2 | Modern ARM64 Linux (e.g., AWS Graviton) |
| **macOS** | `x86_64-apple-darwin` | Tier 1 | Intel-based Macs |
| | `aarch64-apple-darwin`| Tier 1 | Apple Silicon Macs (M1/M2/M3+) |
| **Windows**| `x86_64-pc-windows-msvc` | Tier 1 | 64-bit MSVC toolchain |
| | `i686-pc-windows-msvc` | Tier 1 | 32-bit MSVC toolchain |
| | `x86_64-pc-windows-gnu` | Tier 1 | 64-bit MinGW toolchain |

**Note**: As of Rust 1.78, all Windows targets require a minimum of Windows 10.

### 3. Build Process

Cross-compilation is managed via the included `Makefile`, which automates building for all target platforms and architectures.

1.  **Install Targets**: Run `make install-targets` to add all required Rust targets via `rustup`.
2.  **Build All**: Run `make build-all-platforms` to compile the library for every target.
3.  **Artifacts**: Compiled libraries (`.so`, `.dylib`, `.dll`) will be organized in the `dist/` directory.

For simplified "zero-setup" cross-compilation, the `cross` tool is recommended, especially in CI/CD environments.

### 4. Python Package Distribution
- Wheel distribution with pre-compiled native libraries
- Platform-specific wheels for major platforms
- Source distribution fallback with local compilation
- PyPI publication with automated CI/CD

### 3. Development Workflow
- Cargo workspace integration for unified building
- Python virtual environment setup automation
- Integrated testing across Rust and Python
- Documentation generation from both Rust docs and Python docstrings

## Performance Requirements

### 1. FFI Overhead Targets
- Message pack/unpack: <100ns FFI overhead vs native Rust
- Asset/ticker/provider resolution: <10μs FFI overhead (resolution time dominated by Rust logic)
- Function call overhead: <50ns per FFI function call
- Type conversion overhead: <200ns for complex types (Asset, Ticker)

### 2. Memory Usage
- Minimal Python wrapper object overhead
- Direct buffer passing for pack/unpack operations
- No Python-side caching (leverage Rust's optimizations)
- Platform-appropriate library loading (no memory leaks)

## Security Considerations

### 1. Memory Safety
- No unsafe memory access in FFI boundary
- Proper buffer length validation
- Defensive programming against malformed inputs
- Resource cleanup guarantees

### 2. Input Validation
- Symbol string sanitization
- Numeric range validation
- Message structure integrity checks
- Error propagation without information leakage

## Future Extensions

### 1. Additional Language Bindings
- Node.js/Deno/Bun bindings following the same FFI pattern
- Java JNI wrapper
- C# P/Invoke wrapper  
- Go CGO wrapper

### 2. Enhanced Python Features
- Optional numpy array integration for bulk message processing
- Asyncio-compatible async wrappers for I/O bound operations
- Integration with popular data analysis libraries (pandas, polars)
- Optional high-level trading strategy helpers

---

**Implementation Priority:** Python wrapper serves as the reference implementation for all subsequent language bindings, establishing patterns and best practices for the broader FFI ecosystem.
