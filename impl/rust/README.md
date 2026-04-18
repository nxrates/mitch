# MITCH Rust Reference Implementation

**Official Rust implementation of the MITCH (Moded Individual Trade Clearing and Handling) protocol.**

## Overview

This crate implements the MITCH binary protocol for ultra-low latency market data. It serves as the reference implementation and provides:

- High-performance Rust library
- FFI-ready core for Python, Node.js, MQL4, and C/C++
- Executable protocol specification
- Dynamic library builds for all major platforms and targets (see below)

## Features

- **Ultra-low latency**: Zero-copy, direct memory casting
- **Fire-and-forget**: Non-blocking pub/sub, no acknowledgments
- **Complete protocol**: All MITCH message types (Trade, Order, Tick, Index, OrderBook)
- **Cross-platform**: Consistent Little-Endian encoding
- **No dependencies**: Core has zero external dependencies
- **Memory safe**: Rust safety, optimized hot paths
- **Market provider resolution**: Fuzzy string matching for exchange/broker names
- **Dynamic library output**: Automated building of `.so`, `.dll`, and `.dylib` for all supported targets
- **FFI-ready**: C-compatible interface

## Building Dynamic Libraries

MITCH provides automated scripts and Makefile targets to build dynamic shared libraries for all supported platforms and architectures. This includes:

- Linux (`.so`), macOS (`.dylib`), and Windows (`.dll`) outputs
- 32-bit and 64-bit targets, including ARM and x86
- Output artifacts organized by target for easy integration

To build all dynamic libraries, use:
