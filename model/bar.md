# MITCH Bar Specification

*Part of the [MITCH Protocol](./overview.md) | Message Type: `'k'`*

## Overview

Bar (`k`) is the canonical enriched bar format. At 96 bytes (64B OHLCV + 32B microstructure), it supports both time-based bars (klines) and price-based bars (renko) with a unified layout. The first 64B block holds core OHLCV + timing; the trailing 32B block holds microstructure features.

## Message Structure (96 bytes)

Timestamps are u48 MITCH mts (16 us ticks since 2010-01-01), stored as 6
little-endian bytes. Decode via `mitch::timestamp::{to_epoch_ms, to_epoch_us,
to_epoch_ns}`.

### Cache Line 1 -- Core OHLCV + timing (64 bytes)

| Field       | Offset | Size | Type      | Description                                   |
|-------------|--------|------|-----------|-----------------------------------------------|
| Open TS     | 0      | 6    | `[u8; 6]` | u48 LE mts (16 us ticks since 2010)           |
| Close TS    | 6      | 6    | `[u8; 6]` | u48 LE mts (16 us ticks since 2010)           |
| Open        | 12     | 8    | `f64`     | Opening price                                 |
| High        | 20     | 8    | `f64`     | Highest price                                 |
| Low         | 28     | 8    | `f64`     | Lowest price                                  |
| Close       | 36     | 8    | `f64`     | Closing price                                 |
| VBid        | 44     | 4    | `u32`     | Cumulative bid volume (inherited Index units) |
| VAsk        | 48     | 4    | `u32`     | Cumulative ask volume (inherited Index units) |
| Tick Count  | 52     | 4    | `u32`     | # ingested messages (Index-level, not raw)    |
| _pad        | 56     | 8    | `[u8; 8]` | Padding to 64B                                |

### Microstructure section (32 bytes)

| Field            | Offset | Size | Type       | Description                                            |
|------------------|--------|------|------------|--------------------------------------------------------|
| Realized Var     | 64     | 4    | `f32`      | Σ (log(mid_t / mid_{t-1}))² (canonical HF estimator)   |
| Bipower Var      | 68     | 4    | `f32`      | (π/2) · Σ \|r_t\|·\|r_{t-1}\| (jump-robust)            |
| Drift            | 72     | 4    | `f32`      | OLS slope × duration / close (dimensionless)           |
| Vol Imbalance    | 76     | 4    | `f32`      | Σ sign(r_t) × (vbid+vask)_t / total_vol (signed OFI)   |
| Avg Spread bps   | 80     | 4    | `f32`      | mean((ask - bid) / mid) × 1e4                          |
| Max Abs Return   | 84     | 4    | `f32`      | max \|log(mid_t/mid_{t-1})\| (tail / jump)             |
| Avg CI ubp       | 88     | 2    | `u16`      | Mean inherited `Index.ci_ubp`, sqrt-compressed         |
| Reject Rate      | 90     | 2    | `u16`      | rejected / (accepted + rejected) × 65535               |
| Kind             | 92     | 1    | `u8`       | 0=kline, 1=renko, 2=dib, 3=tib                         |
| Reserved         | 93     | 3    | `[u8; 3]`  | Reserved (zero)                                        |

`jump_var ≈ max(realized_var - bipower_var, 0)` decomposes total variation into
continuous + jump components (Barndorff-Nielsen & Shephard 2004).

## Bar Types

### Time-Based (Kline)

Standard OHLCV candle where `open_ts` and `close_ts` define the time bucket. Duration (ms) = `to_epoch_ms(close_ts) - to_epoch_ms(open_ts)`. Stored `kind = 0`.

### Price-Based (Renko)

Fixed-size price bricks. The OHLC fields encode the brick and its wick. Stored `kind = 1`:

- **Bullish** (close > open): `high == close` (no upper wick), `low` = wick
- **Bearish** (close < open): `low == close` (no lower wick), `high` = wick

## File Format

Binary bar files (`.bars`) are flat arrays of 96-byte Bar records with no header. Record count = `file_size / 96`. Supports zero-copy mmap access via `bytemuck`.

## Usage

```rust
use mitch::{Bar, timestamp};

// Convert epoch ms to MITCH mts for construction
let open_mts  = timestamp::from_epoch_ms(open_ms);
let close_mts = timestamp::from_epoch_ms(close_ms);

// Minimal bar (enrichment fields zeroed, kind=0 kline by default)
let bar = Bar::new_ohlcv(open_mts, close_mts, open, high, low, close, vbid, vask, tick_count);

// Derived (helpers internally decode u48)
let dur_ms = bar.duration_ms();
let ret    = bar.log_return();
```
