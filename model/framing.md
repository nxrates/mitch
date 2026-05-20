# MITCH Framing Specification

*Part of the [MITCH Protocol](./overview.md) | See [Messaging](../messaging.md)*

## Frame Layout

Every MITCH message on the wire or on disk is a **frame**: `[MitchHeader 16B][Body x count]`. The header carries message type, provider ID, timestamp, batch count, flags, sequence, and reserved padding. Body types never embed their own timestamps.

```text
[  MitchHeader 16B ][ Body 0 ][ Body 1 ]...[ Body N-1 ]
|-- type_provider u16 --|  (low 4b = msg code, bits 4..16 = provider_id)
|-- timestamp u48 ------|
|-- count u8 -----------|
|-- flags u8 -----------|
|-- sequence u16 -------|
|-- _reserved [u8; 4] --|
```

### Wire (Streaming)

- `count` may be 1..255, enabling batch transmission
- Timestamp = 16us ticks since 2010-01-01T00:00:00Z

### File (Storage)

- `count = 1` per frame for mmap compatibility (fixed stride)
- Same timestamp encoding as wire

## Timestamp Encoding

**u48 = 16us ticks since 2010-01-01T00:00:00Z**

| Property   | Value                        |
|------------|------------------------------|
| Resolution | 16 microseconds              |
| Epoch      | 2010-01-01T00:00:00Z         |
| Overflow   | ~2152 (142 years)            |
| Encode     | `(epoch_us - EPOCH_2010) >> 4` |
| Decode     | `(ticks << 4) + EPOCH_2010`    |

Single-instruction shift-based codec. See `mitch::timestamp` module.

## Concrete Frame Types

### TradeFrame (40 bytes)

`[MitchHeader 16B][Trade 24B]`

| Field  | Offset | Size | Description              |
|--------|--------|------|--------------------------|
| Header | 0      | 16   | MitchHeader (type = `t`) |
| Body   | 16     | 24   | Trade                    |

### TickFrame (48 bytes)

`[MitchHeader 16B][Tick 32B]`

| Field  | Offset | Size | Description              |
|--------|--------|------|--------------------------|
| Header | 0      | 16   | MitchHeader (type = `s`) |
| Body   | 16     | 32   | Tick                     |

### IndexFrame (56 bytes)

`[MitchHeader 16B][Index 40B]`

| Field  | Offset | Size | Description              |
|--------|--------|------|--------------------------|
| Header | 0      | 16   | MitchHeader (type = `i`) |
| Body   | 16     | 40   | Index                    |

### BarFrame (112 bytes)

`[MitchHeader 16B][Bar 96B]`

| Field  | Offset | Size | Description              |
|--------|--------|------|--------------------------|
| Header | 0      | 16   | MitchHeader (type = `k`) |
| Body   | 16     | 96   | Bar                      |

### HeartbeatFrame (32 bytes)

`[MitchHeader 16B][Heartbeat 16B]`

| Field  | Offset | Size | Description              |
|--------|--------|------|--------------------------|
| Header | 0      | 16   | MitchHeader (type = `h`) |
| Body   | 16     | 16   | Heartbeat                |

Body: `ticker u64` (0 = feed-wide, else per-symbol), `msg_count u32` (data frames emitted since the last heartbeat, wraps at `u32::MAX`), `_pad [u8; 4]`. Consumers diff successive `msg_count` values to quantify gaps between beats; the header `sequence` field tracks gaps between the heartbeats themselves.

## File Format

Binary frame files are flat arrays of fixed-size frame records with no file-level header. Record count = `file_size / frame_size`.

```text
[TickFrame 0][TickFrame 1][TickFrame 2]...
|--- 48B ---|--- 48B ---|--- 48B ---|
```

Supports zero-copy access via `mmap` + `bytemuck::cast_slice::<u8, TickFrame>`.

## Usage

```rust
use mitch::{Tick, TickFrame, timestamp};

let ticks = timestamp::from_epoch_ms(1_744_364_200_000);
let tick = Tick::new_unchecked(ticker_id, 100.0, 100.05, 500, 600);
let frame = TickFrame::new(ticks, tick);

let epoch_ms = timestamp::to_epoch_ms(frame.timestamp());
let mid = frame.mid_price();

// Zero-copy file I/O (bytemuck)
let bytes: &[u8] = bytemuck::cast_slice(&frames);
let back: &[TickFrame] = bytemuck::cast_slice(bytes);
```
