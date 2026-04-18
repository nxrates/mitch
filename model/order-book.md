# MITCH Order Book Specification

*Part of the [MITCH Protocol](./overview.md) | Message Type: `'b'` | See [Messaging Architecture](./messaging.md)*

## Overview

OrderBook messages (`b`) provide comprehensive order book depth (level 2) within adaptive bins for efficient liquidity aggregation.

## Message Structure (2,072 bytes)

| Field       | Offset | Size    | Type    | Description                                |
|-------------|--------|---------|---------|--------------------------------------------|
| Ticker ID   | 0      | 8       | `u64`   | See [ticker.md](ticker.md) for encoding details |
| Mid Price   | 8      | 8       | `f64`   | Current mid market price                   |
| Bin Aggregator | 16  | 1       | `u8`    | Bin aggregation method identifier          |
| Padding     | 17     | 7       | `u8[7]` | Padding to 24 bytes                        |
| Bids        | 24     | 1,024   | `Bin[128]` | Bid levels (-128 to -1 from mid)       |
| Asks        | 1,040  | 1,024   | `Bin[128]` | Ask levels (0 to 127 from mid)          |

## Bin Structure (8 bytes)

| Field  | Offset | Size | Type  | Description                 |
|--------|--------|------|-------|-----------------------------|
| Count  | 0      | 4    | `u32` | Number of orders in bin     |
| Volume | 4      | 4    | `u32` | Total volume to bin boundary|

## Aggregation Methods

### BinAggregator Types
- `0`: DEFAULT_LINGAUSSIAN - Linear + Gaussian growth (bell-shaped)
- `1`: DEFAULT_LINGEOFLAT - Linear + flattened geometric 
- `2`: DEFAULT_BILINGEO - Bi-linear + geometric on edges
- `3`: DEFAULT_TRILINEAR - Tri-linear with steeper edges

The `bin_aggregator` field determines how price levels are distributed around the mid price. Each function is optimized for different market conditions and volatility profiles.

NB: According to modern market theory, all of MITCH's default order book aggregators are U-shaped: the market depth is considered more relevant and informative close to the market current mid-price, therefore the bin steps are always increasing towards the last bin (127).
This way, MITCH's order book snapshots are designed to be extremely informative around mid price (eg. intra-spread and intra-day), but less precise at end of bins distribution.
This is directly reflected by the default linear-gaussian distribution described below, which is recommended for most assets and use cases.

The bin boundaries can be loaded from [../bins], eg:
```csv
bin_id,bin_end
0,0.00001000
1,0.00002000
...
127,200.0
```

### DEFAULT_LINGAUSSIAN (Value: 0)

**Characteristics**: Linear around mid + Gaussian growth (bell-shaped)
- **Min Bin**: 0.00001% -> bids[0] = mid * (1-0.0000001); asks[0] = mid * (1+0.0000001)
- **Max Bin**: 200% -> bids[127] = mid * (1-2), floored at 0; asks[127] = mid * (1+2)
- **Best For**: Any asset, spread and volatility profile
- **Shape**: Tight around mid, exponentially wider away from mid

**Bin Boundaries**:
```
bin(0) -> 0.00001% -> asks[0] aggregates the limit sell from `mid` to `mid * 1.0000001`
bin(1) -> 0.00002%
bin(2) -> 0.00003%
...
bin(14) -> 0.00016000% (transition to exponential)
bin(15) -> 0.00018070% (start of Gaussian growth)
...
bin(40) -> 0.009967% ~1bp -> asks[40] aggregates all the limit sell in the order book from `mid` to `mid * 1.0001`
...
bin(56) -> 0.09930% ~10bp
...
bin(74) -> 1.027% ~1% -> asks[74] aggregates the limit sell from `mid` to `mid * 1.01`
...
bin(95) -> 10.10% ~1% -> asks[95] aggregates the limit sell from `mid` to `mid * 1.1`
...
bin(127) -> 200% -> asks[127] aggregates all the order book limit sell from `mid` to `mid * 3`
```

### DEFAULT_LINGEOFLAT (Value: 1)

**Characteristics**: Linear around mid + flattened geometric growth  
- **Min Bin**: 0.00001%
- **Max Bin**: 200%
- **Best For**: Any asset, spread and volatility profile
- **Shape**: Slightly more uniform than Gaussian, predictable growth

**Key Properties**:
- More consistent bin sizes in mid-range
- Less aggressive expansion than pure Gaussian
- Good balance between precision and coverage

### DEFAULT_BILINGEO (Value: 2)

**Characteristics**: Bi-linear around mid + geometric growth on edges
- **Min Bin**: 0.000025%
- **Max Bin**: 200%
- **Best For**: Most assets, spread and volatility profiles
- **Shape**: Two linear segments then geometric progression

**Bin Distribution**:
- **Linear Segment 1**: Fine granularity near mid (0-25 bins)
- **Linear Segment 2**: Medium granularity in mid-range (26-75 bins)
- **Geometric Segment**: Coarse granularity at edges (76-127 bins)

### DEFAULT_TRILINEAR (Value: 3)

**Characteristics**: Tri-linear with steeper edges
- **Min Bin**: 0.02%
- **Max Bin**: 100% / + 200%
- **Best For**: High volatility and high spread assets
- **Shape**: Three linear segments optimized for extreme movements

**Bin Distribution**:
```
Segment 1 (0-49):   0.02% to 1% (fine, 0.02% steps)
Segment 2 (50-89):  1% to 11% (medium, 0.25% steps) 
Segment 3 (90-127): 11% to 200% (wide, 5% steps)
```

## Aggregation Function Implementation

### Enum Definition
```rust
// Calculate price for bid level (-128 to -1)
pub fn bid_price(&self, level: i8) -> Option<f64> {
    if level >= 0 || level < -128 { return None; }
    let bin_id = (-level - 1) as usize;
    Some(self.calculate_price_for_bin(bin_id, false))
}

// Calculate price for ask level (0 to 127)
pub fn ask_price(&self, level: i8) -> Option<f64> {
    if level < 0 || level > 127 { return None; }
    let bin_id = level as usize;
    Some(self.calculate_price_for_bin(bin_id, true))
}
```

## Performance Characteristics

- **Fixed Size**: Exactly 2KB per order book
- **Cache Friendly**: Single memory block, optimal for CPU cache
- **O(1) Access**: Direct array indexing to any price level
- **Zero-Copy**: Direct memory mapping without parsing
