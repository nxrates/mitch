# MITCH Tick Message Specification

*Part of the [MITCH Protocol](./overview.md) | Message Type: `'s'`*

Tick messages (`s`) are point-in-time level-1 (top of book) snapshots: best bid/ask prices and volumes.

## Wire Layout (32 bytes)

| Field  | Offset | Size | Type  | Description                    |
|--------|--------|------|-------|--------------------------------|
| ticker | 0      | 8    | `u64` | Instrument identifier ([ticker.md](./ticker.md)) |
| bid    | 8      | 8    | `f64` | Best (highest) bid price       |
| ask    | 16     | 8    | `f64` | Best (lowest) ask price        |
| vbid   | 24     | 4    | `u32` | Aggregated bid volume          |
| vask   | 28     | 4    | `u32` | Aggregated ask volume          |

**Framed size**: 48B (16B MitchHeader + 32B body). See [framing.md](./framing.md).

Volumes are in instrument-dependent units (shares, lots, contracts, tokens).

## Derived Calculations

```
mid_price()        = (bid + ask) / 2
spread()           = ask - bid
spread_bps()       = spread / mid * 10000
total_volume()     = vbid + vask
volume_imbalance() = (vask - vbid) / (vask + vbid)    # -1.0 (all bid) .. 1.0 (all ask)
```

## Constraints (enforced by `Tick::validate`)

- `ticker != 0`
- `bid > 0.0`, `ask > 0.0`
- `ask >= bid`

Reference: `impl/rust/src/tick.rs` (`new` validates; `new_unchecked` for trusted wire data).
