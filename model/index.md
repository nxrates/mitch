# MITCH Index Message Specification

*Part of the [MITCH Protocol](./overview.md) | Message Type: `'i'`*

## Wire Layout (40 bytes)

| Field      | Offset | Size | Type    | Description                         |
|------------|--------|------|---------|-------------------------------------|
| ticker     | 0      | 8    | `u64`   | Instrument identifier               |
| bid        | 8      | 8    | `f64`   | Best bid price                      |
| ask        | 16     | 8    | `f64`   | Best ask price                      |
| vbid       | 24     | 4    | `u32`   | Aggregated bid volume               |
| vask       | 28     | 4    | `u32`   | Aggregated ask volume               |
| ci         | 32     | 2    | `u16`   | Confidence interval (micro bps)     |
| tick_count | 34     | 2    | `u16`   | Raw ticks aggregated                |
| confidence | 36     | 1    | `u8`    | Q0.8 freshness `f=byte/255` (FLAG_CONF_FRESHNESS) / legacy active count |
| accepted   | 37     | 1    | `u8`    | Accepted providers                  |
| rejected   | 38     | 1    | `u8`    | Rejected providers                  |
| flags      | 39     | 1    | `u8`    | Bitfield: bit0 heartbeat, bit1 backfill, bit3 conf-freshness |

**Framed size**: 56B (16B MitchHeader + 40B body). See [framing.md](./framing.md).

## Derived Metrics

```
mid()        = (bid + ask) / 2
spread()     = ask - bid
ci_ubp       = (ci / 16.0)^2            # sqrt-compressed decode to micro basis points
ci_price()   = mid * ci_ubp / 1e8       # price-space 1-sigma interval
spread_bps() = spread / mid * 10000
```

`ci` encodes a 1-sigma confidence interval using a sqrt-compressed u16:

```
encoded (u16) = round( sqrt(ci_ubp) * 16.0 )
ci_ubp (f64)  = (encoded / 16.0) ^ 2
```

where `ci_ubp` is in micro basis points of mid (1 ubp = 1e-8 x mid). The compression gives a dynamic range up to ~16.77% of mid before u16 saturation, versus the ~0.065% cap of a flat linear encoding. Reference encode / decode helpers: `nxr_sdk::tdwap::{encode_ci_ubp, decode_ci_ubp}`; `Index::ci_price()` applies the inverse.

## Constraints

- `ask >= bid > 0`
- `ticker != 0`
- `confidence` is INDEPENDENT of `accepted`: when `FLAG_CONF_FRESHNESS` (flags bit 3) is set, `confidence` is Q0.8 freshness (`f = byte/255 ∈ [0,1]`), NOT a provider count, so the old `accepted >= confidence` cross-constraint no longer applies.

## Validation

```rust
pub fn validate(&self) -> Result<(), MitchError> {
    if self.ticker == 0 { return Err("ticker cannot be zero"); }
    if self.bid <= 0.0 || self.ask <= 0.0 { return Err("prices must be positive"); }
    if self.ask < self.bid { return Err("ask must be >= bid"); }
    Ok(())
}
```
