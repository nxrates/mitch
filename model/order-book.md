# MITCH Order Book Specification

*Part of the [MITCH Protocol](./overview.md) | Message Type: `'b'`*

OrderBook messages (`b`) are level-2 depth snapshots aggregated into 128 adaptive bins per side.

## Wire Layout (2,072 bytes)

| Field          | Offset | Size  | Type       | Description                       |
|----------------|--------|-------|------------|-----------------------------------|
| ticker         | 0      | 8     | `u64`      | Instrument identifier ([ticker.md](./ticker.md)) |
| mid_price      | 8      | 8     | `f64`      | Current mid market price          |
| bin_aggregator | 16     | 1     | `u8`       | Bin aggregation method (0-3)      |
| _pad           | 17     | 7     | `[u8;7]`   | Padding to 24 bytes               |
| bids           | 24     | 1,024 | `Bin[128]` | Bid bins, ascending distance below mid |
| asks           | 1,048  | 1,024 | `Bin[128]` | Ask bins, ascending distance above mid |

**Framed size**: 2,088B (16B MitchHeader + 2,072B body). See [framing.md](./framing.md).

### Bin Structure (8 bytes)

| Field       | Offset | Size | Type  | Description                  |
|-------------|--------|------|-------|------------------------------|
| order_count | 0      | 4    | `u32` | Number of orders in bin      |
| volume      | 4      | 4    | `u32` | Total volume to bin boundary |

## Aggregation Methods

| Value | Name | Shape | Min bin | Max bin | Best for |
|-------|------|-------|---------|---------|----------|
| 0 | DEFAULT_LINGAUSSIAN | Linear near mid + Gaussian growth | 0.00001% | 200% | Any asset (recommended default) |
| 1 | DEFAULT_LINGEOFLAT | Linear + flattened geometric | 0.00001% | 200% | More uniform mid-range bins |
| 2 | DEFAULT_BILINGEO | Bi-linear + geometric edges | 0.000025% | 200% | Most assets |
| 3 | DEFAULT_TRILINEAR | Tri-linear, steeper edges | 0.02% | 200% | High volatility / wide spread |

All defaults are U-shaped: bin steps always increase away from mid, so snapshots are most precise intra-spread and coarsest at the tails. Bin `i` on the ask side aggregates all resting sell liquidity from `mid` to `mid * (1 + boundary_i)`; the bid side mirrors below mid (floored at 0).

Bin boundaries are data, not code: one CSV per method in [`../bins/`](../bins/) (`bin_id,bin_end`, 128 rows). The Rust build embeds them as `mitch::constants::BINS`. Sample DEFAULT_LINGAUSSIAN boundaries:

```
bin(0)   -> 0.00001%   (asks[0]: mid to mid * 1.0000001)
bin(40)  -> 0.009967%  ~1 bp
bin(56)  -> 0.09930%   ~10 bp
bin(74)  -> 1.027%     ~1%
bin(95)  -> 10.10%     ~10%
bin(127) -> 200%       (asks[127]: mid to mid * 3)
```

## Constraints (enforced by `OrderBook::validate`)

- `mid_price > 0.0`
- `bin_aggregator <= 3`

## Performance Characteristics

- **Fixed size**: ~2KB per snapshot, single memory block
- **O(1) access**: direct array indexing to any bin
- **Zero-copy**: direct memory mapping without parsing

Reference: `impl/rust/src/order_book.rs`.
