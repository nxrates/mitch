# MITCH Order Message Specification

*Part of the [MITCH Protocol](./overview.md) | Message Type: `'o'`*

Order messages (`o`) represent order lifecycle events: placement, modification, cancellation.

## Wire Layout (32 bytes)

| Field         | Offset | Size | Type      | Description                                    |
|---------------|--------|------|-----------|------------------------------------------------|
| ticker        | 0      | 8    | `u64`     | Instrument identifier ([ticker.md](./ticker.md)) |
| order_id      | 8      | 4    | `u32`     | Unique order identifier (1 to 4,294,967,295; 0 invalid) |
| price         | 12     | 8    | `f64`     | Limit price (limit), trigger price (stop), reference price (market) |
| qty           | 20     | 4    | `u32`     | Order quantity (instrument-dependent units)    |
| type_and_side | 24     | 1    | `u8`      | Bit 0: side (0=Buy, 1=Sell); bits 1-7: order type |
| expiry        | 25     | 6    | `u48` LE  | Expiry as Unix epoch milliseconds; 0 = GTC     |
| _pad          | 31     | 1    | `[u8;1]`  | Padding to 32 bytes                            |

**Framed size**: 48B (16B MitchHeader + 32B body). See [framing.md](./framing.md).

Note: `expiry` uses raw Unix epoch milliseconds, not the 16us-tick header encoding.

## type_and_side Encoding

Order types (bits 1-7):

| Value | Type | Semantics |
|-------|------|-----------|
| `0` | Market | Execute immediately at current market |
| `1` | Limit | Execute only at specified price or better |
| `2` | Stop | Convert to market order when triggered |
| `3` | Cancel | Cancel existing order (only `order_id` and `ticker` required) |

Side (bit 0): `0` = Buy (bid), `1` = Sell (ask).

```rust
use mitch::{combine_type_and_side, extract_order_type, extract_order_side, OrderType, OrderSide};

let tas = combine_type_and_side(OrderType::Limit, OrderSide::Sell); // 0b0000_0011
let (t, s) = (extract_order_type(tas), extract_order_side(tas));
```

## Constraints (enforced by `Order::validate`)

- `ticker != 0`, `order_id != 0`
- Market/Limit/Stop: `price > 0.0` and `qty > 0`
- Cancel: no price/qty requirement

Reference: `impl/rust/src/order.rs`.
