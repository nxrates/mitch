# MITCH Ticker & Asset System

*Part of the [MITCH Protocol](./overview.md) | Core Component*

Instruments are identified hierarchically: an **Asset** (EUR, AAPL, BTC) is a 4-bit asset class + 16-bit ID; a **Ticker** combines two assets (base and quote) with an instrument type and sub-type into a 64-bit ID.

## Part 1: Asset Classification

### Asset Structure (20 bits)

```
[ Asset Class 4b ][ Asset ID 16b ]
```

16 classes x 65,536 assets per class.

### Asset Classes

Source of truth: [`../ids/asset-classes.csv`](../ids/asset-classes.csv). The generated Rust enum (`mitch::AssetClass`) uses the alias column as variant name.

| ID | Class | Alias (enum variant) |
|----|-------|----------------------|
| 0x0 | Equities | EQ |
| 0x1 | Corporate Bonds | CB |
| 0x2 | Sovereign Debt | SD |
| 0x3 | Forex | FX |
| 0x4 | Commodities | CM |
| 0x5 | Real Estate | RE |
| 0x6 | Crypto Assets | CR |
| 0x7 | Private Markets | PM |
| 0x8 | Collectibles | CL |
| 0x9 | Infrastructure | IN |
| 0xA | Indices & Index Products | IP |
| 0xB | Structured Products | SP |
| 0xC | Cash & Equivalents | CE |
| 0xD | Loans & Receivables | LR |
| 0xE-0xF | *Reserved* | |

### Key Asset Examples

Asset IDs are declared in the static tables under [`../ids/`](../ids/overview.md); the CSVs are the single source of truth (examples below are snapshots; regenerate on drift). Aliases live in the pipe-separated `aliases` column: truly-fungible symbols fold into the canonical row's aliases (e.g. `12901,Polygon Ecosystem Token,POL|MATIC`), while distinct-risk assets keep their own rows (e.g. DAI `04801` is deliberately NOT an alias of Sky USDS `14601` since 2026-07-08: the two publish distinct USD pegs).

| Asset Class | ID | Symbol | Description |
|-------------|----|---------|-----------|
| Forex (0x3) | 1301 | EUR | Euro |
| Forex (0x3) | 5001 | USD | US Dollar |
| Equities (0x0) | 831 | AAPL | Apple Inc. |
| CryptoAssets (0x6) | 2701 | BTC | Bitcoin |
| CryptoAssets (0x6) | 17601 | USDT | Tether |
| CryptoAssets (0x6) | 4801 | DAI | Dai (un-aliased from USDS 2026-07-08) |
| Commodities (0x4) | 161 | XAU | Gold |
| Indices (0xA) | 6301 | SPX | S&P 500 Index |

### Asset Encoding/Decoding

```rust
use mitch::{pack_asset, unpack_asset, AssetClass};

let packed: u32 = pack_asset(AssetClass::FX, 1301);   // (class << 16) | id
let (class, id) = unpack_asset(packed);
```

## Part 2: Ticker ID System

### Ticker Structure (64 bits)

```
Bits   | Field              | Size
-------|--------------------|------
63-60  | Instrument Type    | 4
59-56  | Base Asset Class   | 4
55-40  | Base Asset ID      | 16
39-36  | Quote Asset Class  | 4
35-20  | Quote Asset ID     | 16
19-0   | Sub-Type           | 20
```

### Instrument Types (4 bits)

Source of truth: [`../ids/instrument-types.csv`](../ids/instrument-types.csv) (generated enum `mitch::InstrumentType`).

| ID | Type | Alias | Examples |
|----|------|-------|----------|
| `0x0` | Spot | SPOT | FX spot, stock shares, crypto spot |
| `0x1` | Future | FUT | WTI oil, E-mini S&P 500 |
| `0x2` | Forward | FWD | FX forwards, commodity forwards |
| `0x3` | Swap | SWAP | IRS, CCS |
| `0x4` | Perpetual Swap | PERP | BTC-PERP, ETH-PERP |
| `0x5` | Contract For Difference | CFD | Stock CFDs, commodity CFDs |
| `0x6` | Call Option | CALL | Stock options, FX options |
| `0x7` | Put Option | PUT | Protective puts, hedging |
| `0x8` | Digital Option | DIGI | Touch/no-touch, range binaries |
| `0x9` | Barrier Option | BAR | Knock-in/knock-out options |
| `0xA` | Warrant | WAR | Stock warrants, covered warrants |
| `0xB` | Prediction Contract | PRED | Sports betting, election markets |
| `0xC` | Fund or Trust | FUND | ETFs, mutual funds, trusts |
| `0xD` | Structured Product | STRUCT | Autocallables, reverse convertibles |
| `0xE-0xF` | *Reserved* | | |

### Ticker Encoding/Decoding

Reference: `impl/rust/src/ticker.rs`.

```rust
use mitch::{TickerId, AssetClass, InstrumentType};

// EUR/USD spot
let id = TickerId::new(InstrumentType::SPOT, AssetClass::FX, 1301, AssetClass::FX, 5001, 0)?;
assert_eq!(id.raw, 0x0305153138900000);

// Field extraction
let (it, bc, bid) = (id.instrument_type(), id.base_asset_class(), id.base_asset_id());
let (qc, qid, st) = (id.quote_asset_class(), id.quote_asset_id(), id.sub_type());

// Wire form: 8 bytes LE
let bytes = id.pack();
let back = TickerId::unpack(&bytes)?;
```

`TickerId::new` rejects `sub_type > 0xFFFFF` (must fit in 20 bits). Convenience constructors: `forex_ticker`, `crypto_ticker`, `equity_ticker`; batch codecs: `pack_ticker_batch`, `unpack_ticker_batch`.

### Examples

#### EUR/USD Spot Forex
```
Instrument Type: 0x0 (Spot)
Base Asset:      0x3 (Forex) + 1301 (EUR) = 0x30515
Quote Asset:     0x3 (Forex) + 5001 (USD) = 0x31389
Sub-Type:        0x00000

Result: 0x0305153138900000
```

#### AAPL Stock (USD denominated)
```
Instrument Type: 0x0 (Spot)
Base Asset:      0x0 (Equity) + 831 (AAPL) = 0x0033F
Quote Asset:     0x3 (Forex) + 5001 (USD) = 0x31389
Sub-Type:        0x00000

Result: 0x00033F3138900000
```

#### BTC/USDT Perpetual Swap
```
Instrument Type: 0x4 (Perpetual Swap)
Base Asset:      0x6 (Crypto) + 2701 (BTC) = 0x60A8D
Quote Asset:     0x6 (Crypto) + 17601 (USDT) = 0x644C1
Sub-Type:        0x00000

Result: 0x460A8D644C100000
```

## Part 3: Symbol Resolution

Symbol-to-ticker resolution (fuzzy matching, suffix stripping, quote detection) is a consumer-side concern; the reference resolver lives outside this repo (nxr-sdk). This section specifies the normative resolution rules so independent implementations agree.

All processing is **lowercase**: every input is normalized immediately; "BTC", "btc", "Btc" resolve identically. Data is pre-indexed in lowercase hashmaps for O(1) exact lookup, with class-scoped fuzzy matching (confidence score 0.0-1.0) as fallback.

### Prefix/Suffix Stripping

Platform-specific decorations are stripped programmatically, never stored as aliases:

1. **Prefixes**: `^`, `.`, `$`, `#` (e.g. `^SPX`, `.DJI`, `$INDU`, `#GOLD`)
2. **Delimiter-based suffixes** (stripped when following `-`, `_`, `.`, `$`, `^`, `#`): `US`, `M`, `C`, `Z`, `B`, `R`, `D`, `I` (case-insensitive). Examples: `SPX.US`, `DJI_C`, `GOLD$m`, `NDQ-m`
3. **Standalone suffixes** (stripped regardless of delimiter): `USX`, `MINI`, `MICRO`, `CASH`, `SPOT`, `ECN`, `ZERO`. Examples: `SPXmini`, `DJIcash`, `GOLD.spot`

The stripping pass runs **twice** to handle compounds like `XAG.CASH` or `NDQ$MICRO` (first pass removes the descriptive suffix, second the delimiter). A stripped symbol is USD-denominated unless a quote currency is detected.

### Quote Currency Detection

Scan symbol start and end for major quote currencies (separators `/`, `-`, `_`, `.` removed first). Asset IDs are resolved dynamically from the data files, not hardcoded. Priority order by class:

| Asset Class | Quote Priority |
|-------------|---------------|
| Forex | USD, EUR, GBP, JPY, CHF, CAD, AUD |
| Crypto | USDT, USDC, BTC, ETH, USD |
| Equities, Commodities, Indices | USD |

### Base Resolution and Ticker Construction

- Quote detected: resolve the remaining symbol within the target asset class. If nothing remains (input was just "EUR"), the detected asset becomes the base with USD quote.
- No quote detected: resolve the whole cleaned symbol as base; default quote = USD.
- Output: 64-bit ticker ID + base/quote `Asset` objects (see `mitch::{Asset, Ticker, AssetMatch, TickerMatch}` types).

### Resolution Examples

| Input | Asset Class | Processing | Result |
|-------|-------------|------------|---------|
| `"EUR/USD"` | FX | Quote USD at end, resolve EUR | `EUR/USD` spot |
| `"EURUSD"` | FX | Quote USD at end, resolve EUR | `EUR/USD` spot |
| `"SPY.cash"` | EQ | Strip ".cash", default USD quote | `SPY/USD` spot |
| `"GBPJPYmini"` | FX | Strip "mini", quote JPY, resolve GBP | `GBP/JPY` spot |
| `"EUR"` | FX | Quote EUR, empty base, EUR base + USD quote | `EUR/USD` spot |
| `"GOLD"` | CM | No quote, default USD | `GOLD/USD` spot |

### Data Consistency Rules

CSV format and alias conventions: see [`../ids/overview.md`](../ids/overview.md).

## Implementation Reference

- **ID encoding/decoding**: `impl/rust/src/ticker.rs` (`TickerId`, `pack_asset`/`unpack_asset`)
- **Shared types**: `impl/rust/src/common.rs`, `impl/rust/src/market_providers.rs`
- **Generated constants**: `impl/rust/build.rs` compiles `ids/*.csv` + `bins/*.csv` into `mitch::constants` (enums, data arrays, `Resolver` exact-match helper)
- **Tests**: `impl/rust/tests/ticker_test.rs`
