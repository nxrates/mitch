# MITCH Tick Message Specification

*Part of the [MITCH Protocol](./overview.md) | Message Type: `'s'` | See [Messaging Architecture](./messaging.md)*

## Overview

Tick messages (`s`) provide point-in-time bid/ask ticker snapshots representing the best prices and activity in a market. They capture the essential level-1 (top of book) data.

## Message Structure (32 bytes)

| Field       | Offset | Size | Type  | Description                    |
|-------------|--------|------|-------|--------------------------------|
| Ticker ID   | 0      | 8    | `u64` | See [ticker.md](ticker.md) for encoding details |
| Bid Price   | 8      | 8    | `f64` | Best (highest) available bid price |
| Ask Price   | 16     | 8    | `f64` | Best (lowest) available ask price |
| Bid Volume  | 24     | 4    | `u32` | Volume available at best bid |
| Ask Volume  | 28     | 4    | `u32` | Volume available at best ask |

## Field Specifications

### Ticker ID (8 bytes)
**Reference**: [ticker.md](ticker.md) - Complete 8-byte ticker encoding specification

### Bid/Ask Prices (16 bytes total)
- **Type**: `f64` each - Full double precision range, instrument-dependent precision
- **Purpose**: Best available bid (highest) and ask (lowest) prices in the market

### Bid/Ask Volumes (8 bytes total)
- **Type**: `u32` each - Range 0 to 4,294,967,295
- **Units**: Instrument-dependent (shares, lots, contracts, tokens)

## Derived Calculations

### Mid Price
```rust
pub fn mid_price(&self) -> f64 {
    (self.bid_price + self.ask_price) / 2.0
}
```

### Spread Calculations
```rust
pub fn spread(&self) -> f64 {
    self.ask_price - self.bid_price
}

pub fn spread_bps(&self) -> f64 {
    let mid = self.mid_price();
    if mid > 0.0 { (self.spread() / mid) * 10000.0 } else { 0.0 }
}
```

### Volume Analysis
```rust
pub fn total_volume(&self) -> u64 {
    self.bid_volume as u64 + self.ask_volume as u64
}

pub fn volume_imbalance(&self) -> f64 {
    let total = self.total_volume() as f64;
    if total > 0.0 {
        (self.ask_volume as f64 - self.bid_volume as f64) / total
    } else { 0.0 }
}
```

## Validation Rules

```rust
pub fn validate(&self) -> Result<(), &'static str> {
    if self.ticker_id == 0 { return Err("Ticker ID cannot be zero"); }
    if self.bid_price <= 0.0 { return Err("Bid price must be positive"); }
    if self.ask_price <= 0.0 { return Err("Ask price must be positive"); }
    if self.ask_price < self.bid_price { return Err("Ask cannot be less than bid"); }
    Ok(())
}
```

