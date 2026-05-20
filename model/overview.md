# MITCH Protocol Overview

**MITCH (Moded ITCH)** is a transport-agnostic binary protocol for financial market data serialization. Fixed-width fields, zero-copy parsing, little-endian byte order.

## Message Types

| Type | Code | Body Size | Specifications |
|------|------|-----------|----------------|
| Trade | `t` | 24B | [trade.md](./trade.md) |
| Order | `o` | 32B | [order.md](./order.md) |
| Tick | `s` | 32B | [tick.md](./tick.md) |
| Index | `i` | 40B | [index.md](./index.md) |
| Bar | `k` | 96B | [bar.md](./bar.md) |
| OrderBook | `b` | 2072B | [order-book.md](./order-book.md) |

All messages are framed as `[MitchHeader 16B][Body]`. See [framing.md](./framing.md).

## Core Components

- **[Ticker ID System](./ticker.md)**: 8-byte encoding for any financial instrument
- **[Asset Classification](./asset.md)**: Standardized asset class and instrument type system
- **[Messaging Format](../messaging.md)**: Unified header, batching, Channel ID routing
- **[Framing](./framing.md)**: Header + body composition for wire and file I/O

## Data Types & Endianness

All multi-byte fields are **Little-Endian**. Floating points follow IEEE 754.

| Type   | Size | Description                                  |
|--------|------|----------------------------------------------|
| `u8`   | 1    | 8-bit unsigned integer / ASCII char          |
| `u16`  | 2    | 16-bit unsigned integer                      |
| `i16`  | 2    | 16-bit signed integer                        |
| `u32`  | 4    | 32-bit unsigned integer                      |
| `i32`  | 4    | 32-bit signed integer                        |
| `u48`  | 6    | 48-bit unsigned (16us ticks since 2010)      |
| `u64`  | 8    | 64-bit unsigned integer                      |
| `f32`  | 4    | 32-bit IEEE 754 float                        |
| `f64`  | 8    | 64-bit IEEE 754 float                        |

## Performance & Implementation

- **Zero-copy**: `#[repr(C, packed)]` structs with `unsafe { transmute() }` for byte-aligned casting
- **Memory alignment**: Trade/Order/Tick 32B, Index 40B, Bar 96B, OrderBook 2072B
- **Batching**: up to 255 bodies per 16B header (see [messaging](../messaging.md))
- **Architecture**: Little-Endian (x86_64, ARM64, RISC-V)
- **Confidence metrics**: 0-100 quality score (95-100: <10ms latency, <2% rejections)

## Implementation Languages

- **Rust** (`../impl/rust/`) -- Reference implementation
- **TypeScript** (`../impl/mitch.ts`) -- Web/Node.js
- **MQL4** (`../impl/mitch.mq4`) -- MetaTrader
