# MITCH IDS Data Files

This directory contains the core data files used for identifying and categorizing financial instruments within the BTR ecosystem. 

**For complete ticker system documentation, resolution algorithms, and implementation details, see [MITCH Ticker & Asset System](../model/ticker.md).**

## Standardized CSV Format

All asset data files follow a consistent three-column structure:

**Standard Format:**
- `id` - Unique numeric identifier (MITCH ID)
- `name` - Full product/instrument name or description
- `aliases` - Pipe-separated list of trading symbols and alternative names

**Metadata Files Exception:**
- `asset-classes.csv` - Only has `id` and `name` (no aliases needed)
- `instrument-types.csv` - Only has `id` and `name` (no aliases needed)

## Classification Standard Identifiers
- `instrument-types.csv`: Master list of instrument types and their MITCH IDs
- `asset-classes.csv`: Master list of asset classes and their MITCH IDs
- `market-providers.csv`: Exchanges, brokers, and market data providers

## Asset Classes
- `forex.csv`: Fiat currencies with MITCH IDs and ISO codes/aliases
- `commodities.csv`: Commodities with MITCH IDs and trading symbol aliases
- `indices.csv`: Stock indices with MITCH IDs and trading symbol aliases
- `crypto-assets.csv`: Cryptocurrencies and tokens with MITCH IDs and trading symbol aliases
- `equities.csv`: Individual stocks with MITCH IDs and trading symbol aliases

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
