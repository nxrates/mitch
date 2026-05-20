<div align="center">
  <img border-radius="25px" max-height="250px" src="./banner.png" />
  <h1>MITCH</h1>
  <p>
    <strong>Market data, faster than light</strong>
  </p>
  <p>
    <a href="./model/overview.md"><img alt="Docs" src="https://img.shields.io/badge/Docs-212121?style=flat-square&logo=readthedocs&logoColor=white" width="auto"/></a>
    <a href="https://opensource.org/licenses/MIT"><img alt="License" src="https://img.shields.io/badge/license-MIT-000000?style=flat-square&logo=open-source-initiative&logoColor=white&labelColor=4c9c3d" width="auto"/></a>
    <a href="https://t.me/BTRSupply"><img alt="Telegram" src="https://img.shields.io/badge/Telegram-24b3e3?style=flat-square&logo=telegram&logoColor=white" width="auto"/></a>
    <a href="https://twitter.com/BTRSupply"><img alt="X (Twitter)" src="https://img.shields.io/badge/@BTRSupply-000000?style=flat-square&logo=x&logoColor=white" width="auto"/></a>
    </p>
</div>

## Overview

**MITCH (Moded Individual Trade Clearing and Handling)** is a transport-agnostic binary protocol for ultra-low latency market data packing and transmission. Inspired by [NASDAQ's ITCH](https://www.nasdaqtrader.com/content/technicalsupport/specifications/dataproducts/NQTVITCHSpecification.pdf), with altered types and batch packing. See [model/overview.md](./model/overview.md) for the full specification.

## Protocol Specifications

| Component | Description |
|-----------|-------------|
| **[Messaging](./messaging.md)** | Unified 16-byte header, batching, Channel IDs |
| **[Ticker IDs](./model/ticker.md)** | 8-byte encoding for any financial instrument |
| **[Assets](./model/asset.md)** | Standardized asset classification system |
| **[Message Types](./model/overview.md#message-types)** | Trade, Order, Tick, Bar, Index, OrderBook |

## Message Sizes

16-byte header + body (single-entry frame):

| Type | Code | Body | Frame |
|------|------|------|-------|
| Trade | `t` | 24B | 40B |
| Order | `o` | 32B | 48B |
| Tick | `s` | 32B | 48B |
| Index | `i` | 40B | 56B |
| Bar | `k` | 96B | 112B |
| Heartbeat | `h` | 16B | 32B |
| OrderBook | `b` | 2072B | 2088B |

Multi-entry batches: total = 16 + (count × body_size). See [messaging.md](./messaging.md).

## Implementation Languages

| Language | Path | Target |
|----------|------|--------|
| **Rust** | `impl/rust/` | Reference implementation |
| **TypeScript** | `impl/mitch.ts` | Bun, Node, Deno |
| **MQL4** | `impl/mitch.mq4` | MetaTrader 4 |

## Quick Example (Rust)

```rust
let trade = Trade {
    ticker: 0x03006F301CD00000,  // EUR/USD spot
    price: 1.08750,
    volume: 1000000,
    trade_id: [0x40, 0xE2, 0x01], // 123456 as u24 LE
    side: 0, // 0=Buy, 1=Sell
};
let bytes = trade.pack(); // 24 bytes, zero-copy
```

## Contributing

1. Implementations must match the model definitions
2. Maintain cross-language field name consistency
3. Performance first: speed and memory efficiency
4. Validate serialization round-trips across all languages
5. Update relevant spec files with changes

## License

MIT License - see [LICENSE](./LICENSE)

## References

- [Original NASDAQ ITCH Protocol](./itch/v5-specs.pdf)
- [Model Specifications](./model/)
- [Implementation Examples](./impl/examples/)

---

**BTR Supply** | https://btr.supply
