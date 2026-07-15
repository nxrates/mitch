# MITCH Framing Specification

*Part of the [MITCH Protocol](./overview.md) | Header and timestamp encoding: [messaging.md](../messaging.md)*

## Frame Layout

Every MITCH message on the wire or on disk is a **frame**: `[MitchHeader 16B][Body x count]`. The header carries message type, provider ID, timestamp, batch count, flags, sequence, and reserved padding (field layout in [messaging.md](../messaging.md#message-header-16-bytes)). Body types never embed their own timestamps.

- **Wire (streaming)**: `count` may be 1..255, enabling batch transmission.
- **File (storage)**: `count = 1` per frame for mmap compatibility (fixed stride).

## Single-Entry Frame Sizes

| Frame | Body | Total | Rust type |
|-------|------|-------|-----------|
| TradeFrame | Trade 24B | 40B | `Frame<Trade>` alias `TradeFrame` |
| TickFrame | Tick 32B | 48B | `Frame<Tick>` alias `TickFrame` |
| Index frame | Index 40B | 56B | no dedicated alias; `[MitchHeader][Index]` on wire |
| BarFrame | Bar 96B | 112B | `Frame<Bar>` alias `BarFrame` |
| HeartbeatFrame | Heartbeat 16B | 32B | `Frame<Heartbeat>` alias `HeartbeatFrame` |

Reference: `impl/rust/src/frame.rs` (generic `Frame<B>` wrapper, `#[repr(C, packed)]`, Pod + Zeroable).

### HeartbeatFrame (32 bytes)

Heartbeat body (16B, `impl/rust/src/heartbeat.rs`):

| Field     | Offset | Size | Type      | Description                              |
|-----------|--------|------|-----------|------------------------------------------|
| ticker    | 0      | 8    | `u64` LE  | 0 = feed-wide, else per-symbol           |
| msg_count | 8      | 4    | `u32` LE  | Data frames emitted since last beat (wraps at `u32::MAX`) |
| _pad      | 12     | 4    | `[u8; 4]` | Reserved padding                         |

Consumers diff successive `msg_count` values to quantify gaps between beats; the header `sequence` field tracks gaps between the heartbeats themselves.

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
let frame = TickFrame::new(provider_id, ticks, tick);

let epoch_ms = timestamp::to_epoch_ms(frame.timestamp());
let mid = frame.mid_price();

// Zero-copy file I/O (bytemuck)
let bytes: &[u8] = bytemuck::cast_slice(&frames);
let back: &[TickFrame] = bytemuck::cast_slice(bytes);
```
