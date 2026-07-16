# MITCH Index Message Specification

*Part of the [MITCH Protocol](./overview.md) | Message Type: `'i'`*

Index messages (`i`) carry aggregated (multi-provider VWAP composite) market data with confidence and quality metrics. `mid` and `spread` are never stored; they are derived.

## Wire Layout (40 bytes)

| Field      | Offset | Size | Type    | Description                         |
|------------|--------|------|---------|-------------------------------------|
| ticker     | 0      | 8    | `u64`   | Instrument identifier ([ticker.md](./ticker.md)) |
| bid        | 8      | 8    | `f64`   | Best bid price (VWAP composite)     |
| ask        | 16     | 8    | `f64`   | Best ask price (VWAP composite)     |
| vbid       | 24     | 4    | `u32`   | Aggregated bid volume               |
| vask       | 28     | 4    | `u32`   | Aggregated ask volume               |
| ci         | 32     | 2    | `u16`   | Confidence interval, sqrt-compressed micro bps (below) |
| tick_count | 34     | 2    | `u16`   | Raw ticks in aggregation window     |
| confidence | 36     | 1    | `u8`    | freshness percent, 0-100 (`f = byte/255`) when flag bit 3 set; legacy active-provider count otherwise |
| accepted   | 37     | 1    | `u8`    | Accepted providers                  |
| rejected   | 38     | 1    | `u8`    | Rejected providers                  |
| flags      | 39     | 1    | `u8`    | bit 0: heartbeat sentinel; bit 1: historical backfill; bit 3: conf-freshness; bits 2, 4-7 reserved |

**Framed size**: 56B (16B MitchHeader + 40B body). See [framing.md](./framing.md).

## Derived Metrics

```
mid()              = (bid + ask) / 2
spread()           = ask - bid
spread_bps()       = spread / mid * 10000
volume_imbalance() = (vask - vbid) / (vask + vbid)
ci_ubp             = (ci / 16.0)^2            # sqrt-compressed decode to micro basis points
ci_price()         = mid * ci_ubp / 1e8       # price-space 1-sigma interval
```

`ci` encodes a 1-sigma confidence interval using a sqrt-compressed u16:

```
encoded (u16) = round( sqrt(ci_ubp) * 16.0 )
ci_ubp (f64)  = (encoded / 16.0) ^ 2
```

where `ci_ubp` is in micro basis points of mid (1 ubp = 1e-8 x mid). The compression gives a dynamic range up to ~16.77% of mid before u16 saturation, versus the ~0.065% cap of a flat linear encoding. Reference codecs: `mitch::{ci_encode, ci_decode}` (`CI_SCALE = 16.0`); `Index::ci_price()` applies the inverse. Freshness codecs: `mitch::{conf_to_u8, conf_from_u8}`.

## Constraints (enforced by `Index::validate`)

- `ticker != 0`
- `bid`, `ask` finite and `> 0.0`; `ask >= bid`
- `bid`, `ask <= 1e9` (`MAX_PRICE` sanity cap: rejects finite-but-astronomical garbage)
- `spread_bps <= 2000` (20% cap: rejects corrupted feeds, admits the widest illiquid pairs)
- `confidence` is INDEPENDENT of `accepted`: when flag bit 3 (conf-freshness) is set, `confidence` is a freshness percent (0-100), not a provider count, so no `accepted >= confidence` cross-constraint applies.

Reference: `impl/rust/src/index.rs`.
