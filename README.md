<div align="center">
  <img style="border-radius: 25px; max-height: 250px;" src="./banner.png" />
  <h1>MITCH</h1>
  <p>
    <strong>Market data, faster than light</strong>
  </p>
  <p>
    <a href="./model/overview.md"><img alt="Docs" src="https://img.shields.io/badge/Docs-212121?style=flat-square&logo=readthedocs&logoColor=white" width="auto"/></a>
    <a href="https://opensource.org/licenses/MIT"><img alt="License" src="https://img.shields.io/badge/license-MIT-000000?style=flat-square&logo=open-source-initiative&logoColor=white&labelColor=4c9c3d" width="auto"/></a>
    <a href="https://nxrates.com"><img alt="Website" src="https://img.shields.io/badge/nxrates.com-24b3e3?style=flat-square&logo=googlechrome&logoColor=white" width="auto"/></a>
    <a href="https://github.com/nxrates"><img alt="GitHub" src="https://img.shields.io/badge/nxrates-000000?style=flat-square&logo=github&logoColor=white" width="auto"/></a>
    </p>
</div>

## Overview

**MITCH (Moded Individual Trade Clearing and Handling)** is a transport-agnostic binary protocol for ultra-low latency market data packing and transmission. Inspired by [NASDAQ's ITCH](./itch/README.md), with altered types, little-endian encoding, and batch packing.

## Protocol Specifications

| Component | Description |
|-----------|-------------|
| **[Model Overview](./model/overview.md)** | Message types, data types, endianness |
| **[Messaging](./messaging.md)** | 16-byte header, type codes, batching, timestamps, Channel IDs |
| **[Framing](./model/framing.md)** | Frame composition, file format |
| **[Ticker & Asset IDs](./model/ticker.md)** | 8-byte instrument encoding, asset classification |
| **[ID Registries](./ids/overview.md)** | Asset, provider, and instrument-type CSV registries (`ids/`) |
| **[Bin Specs](./bins/)** | Order-book bin layout CSVs consumed by the Rust build (`bins/`) |

Message sizes and per-type field layouts: see [messaging.md](./messaging.md#message-type-codes) and [model/](./model/overview.md#message-types).

## Implementations

| Language | Path | Target |
|----------|------|--------|
| **Rust** | `impl/rust/` | Reference implementation |
| **TypeScript** | `impl/typescript/mitch.ts` | Bun, Node, Deno |
| **Python** | `impl/python/` | CPython >= 3.11 |

Additional ports (C, C++, C#, Go, Java, Zig) live under `impl/`; the Rust crate is the executable specification.

## Quick Example (Rust)

```rust
use mitch::{Trade, OrderSide};

// EUR/USD spot = 0x0305153138900000 (see model/ticker.md)
let trade = Trade::new(0x0305153138900000, 1.08750, 1_000_000, 123456, OrderSide::Buy)?;
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

- [NASDAQ ITCH Protocol summary](./itch/README.md) (official spec on [nasdaqtrader.com](https://www.nasdaqtrader.com/Trader.aspx?id=Totalview2))
- [Model Specifications](./model/)

---

**NX Rates** | https://nxrates.com
