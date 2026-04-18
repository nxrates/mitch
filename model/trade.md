# MITCH Trade Message Specification

*Part of the [MITCH Protocol](./overview.md) | Message Type: `'t'`*

## Wire Layout (24 bytes)

| Field    | Offset | Size | Type      | Description                        |
|----------|--------|------|-----------|------------------------------------|
| ticker   | 0      | 8    | `u64`     | Instrument identifier              |
| price    | 8      | 8    | `f64`     | Execution price                    |
| volume   | 16     | 4    | `u32`     | Executed quantity                   |
| trade_id | 20     | 3    | `[u8;3]`  | Unique trade ID (u24 LE)          |
| side     | 23     | 1    | `u8`      | `0`: Buy, `1`: Sell               |

**Framed size**: 32B (8B MitchHeader + 24B body). See [framing.md](./framing.md).

## Field Notes

**trade_id** is a 24-bit unsigned integer stored as 3 bytes little-endian. Range: 0 to 16,777,215. Decode:

```rust
let id = u32::from_le_bytes([trade_id[0], trade_id[1], trade_id[2], 0]);
```

**side**: `0` = aggressor buying (lift the ask), `1` = aggressor selling (hit the bid).

## Constraints

- `ticker != 0`
- `price > 0.0`
- `volume > 0`
- `side` in `{0, 1}`

## Validation

```rust
pub fn validate(&self) -> Result<(), &'static str> {
    if self.ticker == 0 { return Err("ticker cannot be zero"); }
    if self.price <= 0.0 { return Err("price must be positive"); }
    if self.volume == 0 { return Err("volume must be positive"); }
    Ok(())
}
```
