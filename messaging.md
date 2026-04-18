# MITCH Messaging Architecture

*Part of the [MITCH Protocol](./model/overview.md)*

## Unified Message Format

All MITCH messages: fixed 8-byte header followed by an array of typed body structures.

```
[Header 8B][Body 0][Body 1]...[Body N-1]
```

## Message Header (8 bytes)

| Field        | Offset | Size | Type  | Description                        |
|--------------|--------|------|-------|------------------------------------|
| Message Type | 0      | 1    | `u8`  | ASCII character for message type   |
| Timestamp    | 1      | 6    | `u48` | 16us ticks since 2010-01-01T00:00Z |
| Count        | 7      | 1    | `u8`  | Number of body entries (1-255)     |

All multi-byte fields are **Little-Endian**.

## Message Type Codes

| Code | Type | Body Size | Description |
|------|------|-----------|-------------|
| `t`  | [Trade](./model/trade.md) | 24B | Trade executions |
| `o`  | [Order](./model/order.md) | 32B | Order events |
| `s`  | [Tick](./model/tick.md) | 32B | Tick snapshots |
| `i`  | [Index](./model/index.md) | 40B | Aggregated index data |
| `k`  | [Bar](./model/bar.md) | 128B | OHLCV bars |
| `b`  | [OrderBook](./model/order-book.md) | 2072B | Order book snapshots |

## Batching

- **Count = 1**: Single message (Total: 8 + body_size bytes)
- **Count > 1**: Batch of same type (Total: 8 + count * body_size bytes)
- **Maximum**: 255 entries per message

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
| Bar | 128B |
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
| **UDP Multicast** (239.0.42.1:40006) | Cross-host LAN (nxr -> btr-runtime) | ~5-10us | 48B `mitch::Index` frames |
| **TCP** (port 9500) | FX broker MITCH frames | ~100us | Length-prefixed, batched ticks |
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

### TCP MITCH Frames (FX Brokers)

FX prime brokers connect to port 9500 and stream length-prefixed MITCH frames:

```
[4B payload_len LE][payload]
  payload[0]    = frame_type (0x01=ticks, 0x02=heartbeat)
  payload[2..4] = broker_id (u16 LE)
  payload[8..10]= tick_count (u16 LE)
  payload[10..] = tick_count x 41-byte MITCH ticks
```

### WebSocket

JSON-serialized `Index` objects for browser clients and monitoring dashboards. Connect to `ws://nxr-svc:40004/v1/stream?symbols=123,456`.
