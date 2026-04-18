# MITCH Order Message Specification

*Part of the [MITCH Protocol](./overview.md) | Message Type: `'o'` | See [Messaging Architecture](./messaging.md)*

## Overview

Order messages (`o`) represent order lifecycle events in financial markets, capturing order placement, modification, and cancellation events.

## Message Structure (32 bytes)

| Field         | Offset | Size | Type  | Description                                    |
|---------------|--------|------|-------|------------------------------------------------|
| Ticker ID     | 0      | 8    | `u64` | See [ticker.md](ticker.md) for encoding details |
| Order ID      | 8      | 4    | `u32` | **Required** unique order identifier           |
| Price         | 12     | 8    | `f64` | Limit/stop price                               |
| Quantity      | 20     | 4    | `u32` | Order volume/quantity                          |
| Type & Side   | 24     | 1    | `u8`  | Bit 0: Side (0=Buy, 1=Sell), Bits 1-7: Type    |
| Expiry        | 25     | 6    | `u48` | Expiry timestamp (Unix ms) or 0 for GTC        |
| Padding       | 31     | 1    | `u8`  | Padding to 32 bytes                            |

## Order Type and Side Encoding

The `type_and_side` field uses compact bit layout:

```
┌──────────┬────────────────────────────┐
│ Side (1B)│       Order Type (7B)      │
└──────────┴────────────────────────────┘
   Bit 0              Bits 1-7
```

### Order Types (Bits 1-7)
- `0`: Market - Execute immediately at current market
- `1`: Limit - Execute only at specified price or better
- `2`: Stop - Convert to market order when triggered
- `3`: Cancel - Cancel existing order

### Order Sides (Bit 0)
- `0`: Buy (bid)
- `1`: Sell (ask/offer)

## Field Specifications

### Ticker ID (8 bytes)
**Reference**: [ticker.md](ticker.md) - Complete 8-byte ticker encoding specification

### Order ID (4 bytes)
- **Type**: `u32` - Range 1 to 4,294,967,295 (0 reserved)
- **Purpose**: Unique identifier for the order within the system

### Price (8 bytes)
- **Type**: `f64` - Full double precision range
- **Purpose**: Limit price (limit orders), trigger price (stop orders), reference price (market orders)

### Quantity (4 bytes)
- **Type**: `u32` - Range 0 to 4,294,967,295
- **Units**: Instrument-dependent, original order size or remaining size for modifications

### Expiry (6 bytes)
- **Type**: `u48` - Unix timestamp in milliseconds
- **Special**: `0` = Good-Till-Cancel (GTC), `> 0` = specific expiry time

## Utility Functions

```rust
pub fn extract_order_side(type_and_side: u8) -> OrderSide {
    match type_and_side & 0x01 {
        0 => OrderSide::Buy,
        _ => OrderSide::Sell,
    }
}

pub fn extract_order_type(type_and_side: u8) -> OrderType {
    match (type_and_side >> 1) & 0x7F {
        0 => OrderType::Market,
        1 => OrderType::Limit,
        2 => OrderType::Stop,
        3 => OrderType::Cancel,
        _ => OrderType::Market,
    }
}
```

## Validation Rules

```rust
pub fn validate(&self) -> Result<(), &'static str> {
    if self.order_id == 0 { return Err("Order ID cannot be zero"); }
    if self.ticker_id == 0 { return Err("Ticker ID cannot be zero"); }
    
    let order_type = extract_order_type(self.type_and_side);
    match order_type {
        OrderType::Market | OrderType::Limit | OrderType::Stop => {
            if self.price <= 0.0 { return Err("Price must be positive"); }
            if self.quantity == 0 { return Err("Quantity must be positive"); }
        }
        OrderType::Cancel => {} // Only needs valid order ID
    }
    Ok(())
}
```
