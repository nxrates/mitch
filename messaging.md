# MITCH Messaging Architecture

*Part of the [MITCH Protocol](./model/overview.md)*

## Unified Message Format

All MITCH messages: fixed 16-byte header followed by an array of typed body structures.

```
[Header 16B][Body 0][Body 1]...[Body N-1]
```

## Message Header (16 bytes)

| Field          | Offset | Size | Type      | Description                                          |
|----------------|--------|------|-----------|------------------------------------------------------|
| type_provider  | 0      | 2    | `u16` LE  | `[3:0]` = msg type code, `[15:4]` = provider_id (12b)|
| timestamp      | 2      | 6    | `u48` LE  | 16us ticks since 2010-01-01T00:00Z                   |
| count          | 8      | 1    | `u8`      | Number of body entries (1-255)                       |
| flags          | 9      | 1    | `u8`      | `[1:0]` = version, `[7:2]` = reserved                |
| sequence       | 10     | 2    | `u16` LE  | Per-stream sequence number for gap detection        |
| _reserved      | 12     | 4    | `[u8; 4]` | Reserved (future: CRC32, fragmentation, ...)         |

All multi-byte fields are **Little-Endian**. Body alignment: 16B header keeps bodies aligned to 16B for zero-copy SIMD.

Message type codes (4 bits) packed into the low nibble of `type_provider`; see [model/framing.md](./model/framing.md) and the canonical Rust impl at [`mitch::MitchHeader`](./impl/rust/README.md).

## Message Type Codes

| Code | Type | Body Size | Description |
|------|------|-----------|-------------|
| `t`  | [Trade](./model/trade.md) | 24B | Trade executions |
| `o`  | [Order](./model/order.md) | 32B | Order events |
| `s`  | [Tick](./model/tick.md) | 32B | Tick snapshots |
| `i`  | [Index](./model/index.md) | 40B | Aggregated index data |
| `k`  | [Bar](./model/bar.md) | 96B | OHLCV bars (64B OHLCV + 32B microstructure) |
| `b`  | [OrderBook](./model/order-book.md) | 2072B | Order book snapshots |

## Batching

- **Count = 1**: Single message (Total: 16 + body_size bytes)
- **Count > 1**: Batch of same type (Total: 16 + count * body_size bytes)
- **Maximum**: 255 entries per message

Wire totals for single-entry frames:

| Type | Body | Total frame |
|------|------|-------------|
| Trade | 24B | 40B |
| Order, Tick | 32B | 48B |
| Index | 40B | 56B |
| Bar | 96B | 112B |
| Heartbeat | 16B | 32B |
| OrderBook | 2072B | 2088B |

## Timestamp Handling

- **Type**: `u48` (6 bytes, LE)
- **Units**: 16us ticks since 2010-01-01T00:00:00Z
- **Resolution**: 16 microseconds
- **Overflow**: ~2152 (142 years from epoch)
- **Encode**: `(epoch_us - 1_262_304_000_000_000) >> 4`
- **Decode**: `(ticks << 4) + 1_262_304_000_000_000`

```rust
use mitch::timestamp;

let ticks = timestamp::from_epoch_ms(1_744_364_200_000);
let epoch_ms = timestamp::to_epoch_ms(ticks);
```

## Memory Alignment

| Message | Body Size |
|---------|-----------|
| Trade, Order, Tick | 24B, 32B, 32B |
| Index | 40B |
| Bar | 96B |
| OrderBook | 2072B |

## Channel ID System

32-bit Channel ID for pub/sub filtering (Kafka, ZMQ, gRPC, etc.).

### Format (32-bit)

```
[Market Provider ID 16b][Message Type 8b][Padding 8b]
```

- **Market Provider ID** (16 bits): from `market-providers.csv`. Range: 0-65535.
- **Message Type** (8 bits): ASCII MITCH message type (`'t'`, `'o'`, `'s'`, `'i'`, `'k'`, `'b'`).
- **Padding** (8 bits): Reserved (`0x00`).

### Examples

| Channel | Provider ID | Type | Channel ID |
|---------|------------|------|------------|
| Binance Ticks | 0x0065 (101) | `'s'` (0x73) | `0x00657300` |
| NYSE Index | 0x035D (861) | `'i'` (0x69) | `0x035D6900` |

```rust
let channel_id = Channel::generate(00691, b's');  // IBKR + ticks
subscribe_to_ticker_on_channel(ticker_id, channel_id);
```

---

# Transport Integration

MITCH is transport-agnostic. The binary framing works identically over any byte stream or datagram transport.

| Transport | Use Case | Latency | Notes |
|-----------|----------|---------|-------|
| **UDP Multicast** (239.0.42.1:40006) | Cross-host LAN (nxr -> btr-runtime) | ~5-10us | 56B `IndexFrame` (16B header + 40B Index) |
| **TCP** (port 9500) | FX provider MITCH frames | ~100us | Length-prefixed, batched ticks |
| **WebSocket** (port 40004) | Browser clients, monitoring | ~1ms | JSON-serialized snapshots |

### UDP Multicast

40-byte Index datagrams on a multicast group. Requires `hostNetwork: true` in k8s (multicast doesn't cross CNI overlays).

```rust
use nxr_sdk::NxrClient;

let nxr = NxrClient::new("nxr-svc", 40004);
let mut rx = nxr.subscribe(); // joins 239.0.42.1:40006, zero-copy Index frames
while let Ok(idx) = rx.recv().await {
    println!("{} mid={}", idx.ticker, idx.mid());
}
```

### TCP MITCH Frames (FX Providers)

FX prime providers connect to port 9500 and stream **pure canonical MITCH
frames** — no length prefix, no NXR-specific envelope. Each frame is
`[MitchHeader 16B][Body × count]`; the receiver reads 16 bytes, decodes the
header, and then reads `count × body_size` more.

- Ticks: `type_provider` low 4 bits = `3` (`TICK`), body = 32B `mitch::Tick`.
- Heartbeats: `type_provider` low 4 bits = `7` (`HEARTBEAT`), body = 16B
  `mitch::Heartbeat` (total frame = 32B `mitch::HeartbeatFrame`).
- `provider_id` lives in the high 12 bits of `type_provider` as the MITCH
  `provider_id` (0-4095).
- Batches larger than 255 ticks are split across multiple frames that share
  the same `mts` and use consecutive `sequence` values.

### WebSocket

JSON-serialized `Index` objects for browser clients and monitoring dashboards. Connect to `ws://nxr-svc:40004/v1/stream?symbols=123,456`.
