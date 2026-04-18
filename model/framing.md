# MITCH Framing Specification

*Part of the [MITCH Protocol](./overview.md) | See [Messaging](../messaging.md)*

## Frame Layout

Every MITCH message on the wire or on disk is a **frame**: `[MitchHeader 8B][Body x count]`. The header carries message type, timestamp, and batch count. Body types never embed their own timestamps.

```text
[  MitchHeader 8B  ][ Body 0 ][ Body 1 ]...[ Body N-1 ]
|-- type u8 --------|
|-- timestamp u48 --|
|-- count u8 -------|
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

### TradeFrame (32 bytes)

`[MitchHeader 8B][Trade 24B]`

| Field  | Offset | Size | Description              |
|--------|--------|------|--------------------------|
| Header | 0      | 8    | MitchHeader (type = `t`) |
| Body   | 8      | 24   | Trade                    |

### TickFrame (40 bytes)

`[MitchHeader 8B][Tick 32B]`

| Field  | Offset | Size | Description              |
|--------|--------|------|--------------------------|
| Header | 0      | 8    | MitchHeader (type = `s`) |
| Body   | 8      | 32   | Tick                     |

### IndexFrame (48 bytes)

`[MitchHeader 8B][Index 40B]`

| Field  | Offset | Size | Description              |
|--------|--------|------|--------------------------|
| Header | 0      | 8    | MitchHeader (type = `i`) |
| Body   | 8      | 40   | Index                    |

### BarFrame (136 bytes)

`[MitchHeader 8B][Bar 128B]`

| Field  | Offset | Size | Description              |
|--------|--------|------|--------------------------|
| Header | 0      | 8    | MitchHeader (type = `k`) |
| Body   | 8      | 128  | Bar                      |

## File Format

Binary frame files are flat arrays of fixed-size frame records with no file-level header. Record count = `file_size / frame_size`.

```text
[TickFrame 0][TickFrame 1][TickFrame 2]...
|--- 40B ---|--- 40B ---|--- 40B ---|
```

Supports zero-copy access via `mmap` + `bytemuck::cast_slice::<u8, TickFrame>`.

## Usage

```rust
use mitch::{Tick, TickFrame, timestamp};

let ticks = timestamp::from_epoch_ms(1_744_364_200_000);
let tick = Tick::new_unchecked(ticker_id, 100.0, 100.05, 500, 600);
let frame = TickFrame::new(ticks, tick);

let ts_ms = timestamp::to_epoch_ms(frame.timestamp());
let mid = frame.mid_price();

// Zero-copy file I/O (bytemuck)
let bytes: &[u8] = bytemuck::cast_slice(&frames);
let back: &[TickFrame] = bytemuck::cast_slice(bytes);
```
