# MITCH Protocol Overview

**MITCH (Moded ITCH)** is a transport-agnostic binary protocol for financial market data serialization. Fixed-width fields, zero-copy parsing, little-endian byte order.

## Message Types

| Type | Code | Body Size | Specification |
|------|------|-----------|---------------|
| Trade | `t` | 24B | [trade.md](./trade.md) |
| Order | `o` | 32B | [order.md](./order.md) |
| Tick | `s` | 32B | [tick.md](./tick.md) |
| Index | `i` | 40B | [index.md](./index.md) |
| OrderBook | `b` | 2072B | [order-book.md](./order-book.md) |
| Bar | `k` | 96B | [bar.md](./bar.md) |
| Heartbeat | `h` | 16B | [framing.md](./framing.md#heartbeatframe-32-bytes) |

All messages are framed as `[MitchHeader 16B][Body x count]`. Header layout, wire type codes, batching, and timestamps: [messaging.md](../messaging.md). Frame composition and file format: [framing.md](./framing.md).

## Core Components

- **[Ticker & Asset IDs](./ticker.md)**: 8-byte instrument encoding + asset classification
- **[Messaging](../messaging.md)**: Unified header, batching, Channel ID routing
- **[Framing](./framing.md)**: Header + body composition for wire and file I/O

## Data Types & Endianness

All multi-byte fields are **Little-Endian**. Floating points follow IEEE 754.

| Type   | Size | Description                                  |
|--------|------|----------------------------------------------|
| `u8`   | 1    | 8-bit unsigned integer / ASCII char          |
| `u16`  | 2    | 16-bit unsigned integer                      |
| `u32`  | 4    | 32-bit unsigned integer                      |
| `u48`  | 6    | 48-bit unsigned (16us ticks since 2010, see [messaging.md](../messaging.md#timestamp-encoding)) |
| `u64`  | 8    | 64-bit unsigned integer                      |
| `f32`  | 4    | 32-bit IEEE 754 float                        |
| `f64`  | 8    | 64-bit IEEE 754 float                        |

## Performance & Implementation

- **Zero-copy**: `#[repr(C, packed)]` structs cast directly to/from bytes
- **Batching**: up to 255 bodies per 16B header (see [messaging.md](../messaging.md#batching))
- **Architecture**: Little-Endian native (x86_64, ARM64, RISC-V)

## Implementations

- **Rust** (`../impl/rust/`): reference implementation
- **TypeScript** (`../impl/typescript/mitch.ts`): Bun, Node, Deno
- **MQL4** (`../impl/mql4/mitch.mq4`): MetaTrader 4
- Additional ports under `../impl/` (C, C++, C#, Go, Java, Python, Zig)
