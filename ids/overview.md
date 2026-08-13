# MITCH IDS Data Files

This directory contains the data files identifying and categorizing financial instruments. These CSVs are the single source of truth: the Rust build (`impl/rust/build.rs`) compiles them into `mitch::constants` at build time.

**For the ticker/asset encoding and resolution rules, see [MITCH Ticker & Asset System](../model/ticker.md).**

## Standardized CSV Format

All files use three columns:

- `id` (or `mitch_id` in `instrument-types.csv`) - Unique numeric identifier (MITCH ID)
- `name` - Full product/instrument name or description
- `aliases` - Pipe-separated trading symbols and alternative names. In `asset-classes.csv` and `instrument-types.csv` the single alias doubles as the generated Rust enum variant name (e.g. `FX`, `SPOT`); rows with `*Reserved*` markers or empty aliases are skipped by codegen.

## Classification Files
- `asset-classes.csv`: Asset classes (generates `mitch::AssetClass`)
- `instrument-types.csv`: Instrument types (generates `mitch::InstrumentType`)
- `market-providers.csv`: Exchanges, brokers, and market data providers

## Asset Data Files (one per asset class)
- `forex.csv`: Fiat currencies (ISO codes as aliases)
- `commodities.csv`: Commodities
- `indices.csv`: Market indices
- `crypto-assets.csv`: Cryptocurrencies and tokens
- `equities.csv`: Individual stocks
- `sovereign-debt.csv`: Government bonds/bills

## CSV Data Rules

### File Requirements
1. **UTF-8 Encoding**: All data files use UTF-8 encoding for international symbol support
2. **Primary Symbol**: The most commonly used trading symbol should be the first in the aliases list
3. **ISO Code Integration**: Currency ISO codes (USD, EUR, etc.) are stored as aliases in forex.csv
4. **Pipe Separation**: Multiple aliases are separated by `|` character
5. **No Redundant Suffixes**: Common prefixes/suffixes are handled programmatically, not stored as aliases

### Symbol Storage Conventions
- **Normalized Storage**: Store symbols in their cleanest form without platform-specific additions
- **Alias Completeness**: Include major alternative representations for each instrument
- **Case Consistency**: Store aliases in natural case, but remember all processing is lowercase
- **Special Characters**: Use standard symbols; avoid platform-specific encodings

## Integration with External Reference Data

Asset IDs coordinate with external reference data sources:

### Standard Identifiers
- **FIGI Codes**: Bloomberg's Financial Instrument Global Identifier
- **ISIN Codes**: International Securities Identification Number
- **CUSIP Codes**: Committee on Uniform Securities Identification Procedures

### Asset-Specific Identifiers
- **Forex**: ISO 4217 currency codes (stored as aliases)
- **Crypto**: On-chain addresses, ENS/SNS domains, CMC/CoinGecko IDs
- **Commodities**: Exchange-specific symbols (CBOE, LME, NYMEX, ICE)
- **Equities**: Stock exchange ticker symbols and alternative listings

Converting these external identifiers to MITCH IDs enables standardized communication with MITCH-enabled services while maintaining compatibility with existing financial data infrastructure.

## Aliases name the asset, not the wrapper

An alias resolves the ASSET. The instrument type lives in the ticker id's 4 bits
(Spot, Future, CFD, Fund or Trust, ...), so a futures code legitimately aliases the
asset it settles against: `ES` on S&P 500, `FDAX` on DAX, `GC` on Gold. Stripping
those would break resolution for no gain.

An ETF ticker is different. A fund that tracks an index is its own asset, with fees
and tracking error, so it gets its own Fund-typed row rather than an alias on the
index. `SOXX` (PHLX Semiconductor) and `MDY` (S&P 400 MidCap) were removed on that
basis 2026-08-13.
