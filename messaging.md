# MITCH Messaging (Wire Protocol)

*Part of the [MITCH Protocol](./model/overview.md)*

## Unified Message Format

Every MITCH message is a fixed 16-byte header followed by 1 to 255 typed body entries of the same type:

```
[MitchHeader 16B][Body 0][Body 1]...[Body N-1]
```

## Message Header (16 bytes)

Canonical layout (reference: `impl/rust/src/header.rs`):

| Field          | Offset | Size | Type      | Description                                          |
|----------------|--------|------|-----------|------------------------------------------------------|
| type_provider  | 0      | 2    | `u16` LE  | `[3:0]` = msg type code, `[15:4]` = provider_id (12b)|
| timestamp      | 2      | 6    | `u48` LE  | 16us ticks since 2010-01-01T00:00:00Z                |
| count          | 8      | 1    | `u8`      | Number of body entries (1-255)                       |
| flags          | 9      | 1    | `u8`      | `[1:0]` = version, `[7:2]` = reserved                |
| sequence       | 10     | 2    | `u16` LE  | Per-stream sequence number for gap detection         |
| _reserved      | 12     | 4    | `[u8; 4]` | Reserved (future: CRC32, fragmentation, ...)         |

All multi-byte fields are **Little-Endian**. The 16B header keeps bodies aligned to 16B for zero-copy SIMD.

`provider_id` (12 bits, 0-4095) identifies the data source; see `ids/market-providers.csv`.

## Message Type Codes

ASCII codes identify types in APIs; a 4-bit wire code (low nibble of `type_provider`) identifies them on the wire.

| ASCII | Wire code | Type | Body | Single-entry frame |
|-------|-----------|------|------|--------------------|
| `t` | 1 | [Trade](./model/trade.md) | 24B | 40B |
| `o` | 2 | [Order](./model/order.md) | 32B | 48B |
| `s` | 3 | [Tick](./model/tick.md) | 32B | 48B |
| `i` | 4 | [Index](./model/index.md) | 40B | 56B |
| `b` | 5 | [OrderBook](./model/order-book.md) | 2072B | 2088B |
| `k` | 6 | [Bar](./model/bar.md) | 96B | 112B |
| `h` | 7 | Heartbeat (see [framing.md](./model/framing.md#heartbeatframe-32-bytes)) | 16B | 32B |

## Batching

- Total message size = `16 + count * body_size` bytes; max 255 entries per message.
- Batches larger than 255 entries are split across consecutive frames sharing the same timestamp and using consecutive `sequence` values.
- Stream receivers read 16 bytes, decode the header, then read `count * body_size` more. Frames carry no length prefix or envelope.

## Timestamp Encoding

**u48 = 16us ticks since 2010-01-01T00:00:00Z** (reference: `impl/rust/src/timestamp.rs`).

| Property   | Value                        |
|------------|------------------------------|
| Resolution | 16 microseconds (62,500 ticks/s) |
| Epoch      | 2010-01-01T00:00:00Z (Unix 1,262,304,000) |
| Overflow   | ~2152 (142 years from epoch) |
| Encode     | `(epoch_us - 1_262_304_000_000_000) >> 4` |
| Decode     | `(ticks << 4) + 1_262_304_000_000_000` |

Pre-2010 inputs saturate to tick 0.

```rust
use mitch::timestamp;

let ticks = timestamp::from_epoch_ms(1_744_364_200_000);
let epoch_ms = timestamp::to_epoch_ms(ticks);
```

## Channel ID System

32-bit Channel ID for pub/sub filtering (Kafka, ZMQ, gRPC, etc.). Reference: `impl/rust/src/channel.rs`.

### Format (32-bit)

```
[Market Provider ID 16b][Message Type 8b][Padding 8b]
```

- **Market Provider ID** (16 bits): from `ids/market-providers.csv`. Range: 0-65535.
- **Message Type** (8 bits): ASCII MITCH message type. Valid: `'t'`, `'o'`, `'s'`, `'i'`, `'b'`.
- **Padding** (8 bits): Reserved (`0x00`).

### Examples

| Channel | Provider ID | Type | Channel ID |
|---------|------------|------|------------|
| Binance Ticks | 0x0065 (101) | `'s'` (0x73) | `0x00657300` |
| Interactive Brokers Index | 0x02B3 (691) | `'i'` (0x69) | `0x02B36900` |

```rust
let channel = mitch::ChannelId::new(691, 's'); // IBKR + ticks
```

## Transport Integration

MITCH is transport-agnostic: the same framing works over any byte stream or datagram transport (UDP multicast, TCP, WebSocket, IPC, files). Datagram transports carry one frame per datagram; stream transports concatenate frames back-to-back (see Batching above for the read loop). Producers should interleave [Heartbeat](./model/framing.md#heartbeatframe-32-bytes) frames so consumers can detect stale feeds and quantify gaps.
