# MITCH Bar Specification

*Part of the [MITCH Protocol](./overview.md) | Message Type: `'k'`*

## Overview

Bar (`k`) is the canonical enriched bar format. At 128 bytes (2 cache lines), it supports both time-based bars (klines) and price-based bars (renko) with a unified layout. The first cache line holds core OHLCV + timing; the second holds microstructure features.

## Message Structure (128 bytes)

### Cache Line 1 -- Core OHLCV (64 bytes)

| Field       | Offset | Size | Type  | Description                          |
|-------------|--------|------|-------|--------------------------------------|
| Open Time   | 0      | 8    | `i64` | Bar open (ms since epoch)            |
| Close Time  | 8      | 8    | `i64` | Bar close (ms since epoch)           |
| Open        | 16     | 8    | `f64` | Opening price                        |
| High        | 24     | 8    | `f64` | Highest price                        |
| Low         | 32     | 8    | `f64` | Lowest price                         |
| Close       | 40     | 8    | `f64` | Closing price                        |
| VBid        | 48     | 4    | `u32` | Cumulative bid volume                |
| VAsk        | 52     | 4    | `u32` | Cumulative ask volume                |
| Spread      | 56     | 4    | `f32` | Average spread ratio                 |
| Tick Count  | 60     | 4    | `u32` | Ticks in bar                         |

### Cache Line 2 -- Microstructure (64 bytes)

| Field              | Offset | Size | Type  | Description                     |
|--------------------|--------|------|-------|---------------------------------|
| N Buys             | 64     | 4    | `u32` | Buy-side tick count             |
| N Sells            | 68     | 4    | `u32` | Sell-side tick count            |
| Velocity           | 72     | 4    | `f32` | Normalized activity             |
| Dispersion         | 76     | 4    | `f32` | Normalized volatility           |
| Drift              | 80     | 4    | `f32` | Normalized trend slope          |
| VWAP Dev           | 84     | 4    | `f32` | VWAP deviation from close       |
| Kyle Lambda        | 88     | 4    | `f32` | Price impact coefficient        |
| OFI                | 92     | 4    | `f32` | Order flow imbalance            |
| H/L Ratio          | 96     | 4    | `f32` | High/Low ratio                  |
| Tick Efficiency     | 100    | 4    | `f32` | Price path efficiency           |
| Log Volume         | 104    | 4    | `f32` | ln(total_volume)                |
| Theta Trigger      | 108    | 4    | `f32` | Information-bar trigger value   |
| Expected Imbalance | 112    | 4    | `f32` | E[tick imbalance] (EWMA)        |
| Expected Ticks     | 116    | 4    | `f32` | E[ticks per bar] (EWMA)         |
| Max Trade USD      | 120    | 4    | `f32` | Largest single trade ($)        |
| Reserved           | 124    | 4    | `u32` | Reserved (zero)                 |

## Bar Types

### Time-Based (Kline)

Standard OHLCV candle where `open_time` and `close_time` define the time bucket. Duration = `close_time - open_time`.

### Price-Based (Renko)

Fixed-size price bricks. The OHLC fields encode the brick and its wick:

- **Bullish** (close > open): `high == close` (no upper wick), `low` = wick
- **Bearish** (close < open): `low == close` (no lower wick), `high` = wick

## File Format

Binary bar files (`.bars`) are flat arrays of 128-byte Bar records with no header. Record count = `file_size / 128`. Supports zero-copy mmap access via `bytemuck`.

## Usage

```rust
use mitch::Bar;

// Minimal bar (enrichment fields zeroed)
let bar = Bar::new_ohlcv(open_ms, close_ms, open, high, low, close, vbid, vask, tick_count);

// Derived
let dur = bar.duration_ms();
let ret = bar.log_return();
let imb = bar.volume_imbalance();
```
