# MITCH Ticker & Asset System

*Part of the [MITCH Protocol](./overview.md) | Core Component*

## Overview

The MITCH protocol uses a hierarchical identification system for financial instruments, starting with **Assets** (individual instruments like EUR, AAPL, BTC) and combining them into **Tickers** (trading pairs like EUR/USD, AAPL/USD, BTC/USDT). This system enables efficient encoding, fast lookups, and standardized identification across all financial markets.

## Part 1: Asset Classification System

### Asset Structure (20 bits)
```
┌─────────────┬─────────────────────┐
│ Asset Class │ Asset ID            │
│ (4 bits)    │ (16 bits)           │
└─────────────┴─────────────────────┘
```

Each asset combines a **4-bit asset class** with a **16-bit unique identifier**, providing 16 classes × 65,536 assets per class.

### Asset Classes

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AssetClass {
    Equities = 0x0,           // Publicly traded stocks
    CorporateBonds = 0x1,     // Corporate debt securities  
    SovereignDebt = 0x2,      // Government bonds/bills
    Forex = 0x3,              // Fiat currencies
    Commodities = 0x4,        // Physical goods/raw materials
    RealEstate = 0x5,         // Property investments/REITs
    CryptoAssets = 0x6,       // Cryptocurrencies/tokens
    PrivateMarkets = 0x7,     // Private equity/VC investments
    Collectibles = 0x8,       // Art/antiques/collectibles
    Infrastructure = 0x9,     // Infrastructure investments
    Indices = 0xA,            // Market indices
    StructuredProducts = 0xB, // Complex financial instruments
    CashEquivalents = 0xC,    // Cash/money market instruments
    LoansReceivables = 0xD,   // Loan instruments
    // 0xE-0xF reserved
}
```

### Key Asset Examples

| Asset Class | ID | Symbol | Description |
|-------------|----|---------|-----------| 
| Forex (0x3) | 111 | EUR | Euro |
| Forex (0x3) | 461 | USD | US Dollar |
| Equities (0x0) | 831 | AAPL | Apple Inc. |
| CryptoAssets (0x6) | 2701 | BTC | Bitcoin |
| CryptoAssets (0x6) | 17601 | USDT | Tether |
| Commodities (0x4) | 161 | GOLD | Gold |
| Indices (0xA) | 671 | SPX | S&P 500 Index |

### Asset Encoding/Decoding

```rust
// Pack asset into 32-bit value
pub fn pack_asset(asset_class: AssetClass, asset_id: u16) -> u32 {
    ((asset_class as u32) << 16) | (asset_id as u32)
}

// Unpack asset from 32-bit value
pub fn unpack_asset(packed_asset: u32) -> (AssetClass, u16) {
    let class_id = ((packed_asset >> 16) & 0xF) as u8;
    let asset_id = (packed_asset & 0xFFFF) as u16;
    (AssetClass::from(class_id), asset_id)
}
```

## Part 2: Ticker ID System

### Ticker Structure (64 bits)
A **Ticker** combines two assets (base and quote) with instrument type and sub-type information:

```
┌───────────────┬───────────────────────┬───────────────────────┬───────────────────────┐
│ Inst. Type    │ Base Asset (class+id) │ Quote Asset (class+id)│ Sub-Type              │
│ (4 bits)      │ 20 bits    (4+16 bits)│ 20 bits    (4+16 bits)│ (20 bits)             │
└───────────────┴───────────────────────┴───────────────────────┴───────────────────────┘
```

### Instrument Types (4 bits)

| ID | Type | Description | Use Cases |
|----|------|-------------|-----------|
| `0x0` | Spot | Direct asset trading | FX spot, stock shares, crypto spot |
| `0x1` | Future | Standardized futures contract | WTI oil, E-mini S&P 500 |
| `0x2` | Forward | Custom forward contract | FX forwards, commodity forwards |
| `0x3` | Swap | Interest rate or currency swap | IRS, CCS |
| `0x4` | Perpetual Swap | Crypto perpetual futures | BTC-PERP, ETH-PERP |
| `0x5` | CFD | Contract for difference | Stock CFDs, commodity CFDs |
| `0x6` | Call Option | Call option contract | Stock options, FX options |
| `0x7` | Put Option | Put option contract | Protective puts, hedging |
| `0x8` | Digital Option | Binary/digital option | Touch/no-touch, range binaries |
| `0x9` | Barrier Option | Barrier option contract | Knock-in/knock-out options |
| `0xA` | Warrant | Warrant contract | Stock warrants, covered warrants |
| `0xB` | Prediction Contract | Contract based on predicted outcomes | Sports betting, election markets |
| `0xC` | Structured Product | Multi-component financial instruments | Autocallables, reverse convertibles |
| `0xD-0xF` | *Reserved* | *Reserved for future use* | *Future instrument types* |

### Ticker Encoding/Decoding

```rust
// Generate ticker ID from components
pub fn generate_ticker_id(
    instrument_type: InstrumentType,
    base_class: AssetClass,
    base_id: u16,
    quote_class: AssetClass,
    quote_id: u16,
    sub_type: u32,
) -> Result<u64, &'static str> {
    if sub_type > 0xFFFFF {
        return Err("Sub-type must fit in 20 bits");
    }

    let base_asset = ((base_class as u32) << 16) | (base_id as u32);
    let quote_asset = ((quote_class as u32) << 16) | (quote_id as u32);

    let ticker_id = ((instrument_type as u64) << 60) |
                   ((base_asset as u64) << 40) |
                   ((quote_asset as u64) << 20) |
                   (sub_type as u64);

    Ok(ticker_id)
}

// Extract components from ticker ID
pub fn extract_ticker_components(ticker_id: u64) -> (u8, u8, u16, u8, u16, u32) {
    let instrument_type = ((ticker_id >> 60) & 0xF) as u8;
    let base_asset = ((ticker_id >> 40) & 0xFFFFF) as u32;
    let quote_asset = ((ticker_id >> 20) & 0xFFFFF) as u32;
    let sub_type = (ticker_id & 0xFFFFF) as u32;

    let base_class = ((base_asset >> 16) & 0xF) as u8;
    let base_id = (base_asset & 0xFFFF) as u16;
    let quote_class = ((quote_asset >> 16) & 0xF) as u8;
    let quote_id = (quote_asset & 0xFFFF) as u16;

    (instrument_type, base_class, base_id, quote_class, quote_id, sub_type)
}
```

### Real-World Examples

#### EUR/USD Spot Forex
```
Instrument Type: 0x0 (Spot)
Base Asset:      0x3 (Forex) + 111 (EUR) = 0x3006F
Quote Asset:     0x3 (Forex) + 461 (USD) = 0x301CD  
Sub-Type:        0x00000

Result: 0x03006F301CD00000 (216295034546290688 decimal)
```

#### AAPL Stock (USD denominated)
```
Instrument Type: 0x0 (Spot)
Base Asset:      0x0 (Equity) + 831 (AAPL) = 0x0033F
Quote Asset:     0x3 (Forex) + 461 (USD) = 0x301CD
Sub-Type:        0x00000

Result: 0x00033F301CD00000 (230056771460096 decimal)
```

#### BTC/USDT Perpetual Swap
```
Instrument Type: 0x4 (Perpetual Swap)
Base Asset:      0x6 (Crypto) + 2701 (BTC) = 0x60A8D
Quote Asset:     0x6 (Crypto) + 17601 (USDT) = 0x644C1
Sub-Type:        0x00000

Result: 0x460A8D644C100000 (5043977300425089024 decimal)
```

## Part 3: Symbol Resolution Process

The MITCH system resolves trading symbols into standardized ticker IDs through a sophisticated multi-step process that operates entirely in **lowercase** for maximum performance. All input symbols are normalized to lowercase immediately for consistent and fast processing.

### Asset Resolution Parsing Logic

The BTR system uses a sophisticated multi-step parsing process for individual asset identification.

#### Asset Parsing Process

**Input:** Asset name or symbol (e.g., "APPLE", "btc", "Gold")  
**Asset Class:** Specified target asset class for filtering

**Steps:**
1. **Normalization:** Convert input to lowercase and apply standard normalization
2. **Suffix Stripping:** Apply automatic suffix removal rules (see below)
3. **Exact Match:** Check for exact match in pre-built lowercase index
4. **Fuzzy Matching:** If no exact match, use enhanced similarity scoring against all candidates within the specified asset class
5. **Confidence Scoring:** Return best match with confidence score (0.0-1.0)

**Performance Optimization:** All data is pre-indexed in lowercase hashmaps for O(1) exact lookups and optimized fuzzy search.

### Ticker Resolution Parsing Logic

The ticker resolver takes an **asset class** and **ticker symbol** as input and outputs a complete `Ticker` object with base/quote asset identification.

#### Ticker Parsing Process

**Input:** 
- `asset_class`: Target asset class (e.g., `AssetClass::FX` for forex)
- `symbol`: Raw ticker string (e.g., "EUR/USD", "EURUSD", "SPY.cash", "GBPJPYmini")
- `instrument_type`: Type of instrument (spot, futures, etc.)

**Processing Steps:**

##### 1. Suffix Stripping
Convert to lowercase and strip common prefixes/suffixes:
- **Prefixes:** `^`, `.`, `$`, `#`
- **Delimiter-based suffixes:** `us`, `m`, `c`, `z`, `b`, `r`, `d`, `i` (when following `-`, `_`, `.`, `$`, `^`, `#`)
- **Standalone suffixes:** `usx`, `mini`, `micro`, `cash`, `spot`, `ecn`, `zero`
- **Compound handling:** Run twice to handle cases like "spy.cash" or "ndq$micro"

##### 2. Quote Currency Detection
Scan for major quote currencies at beginning or end of symbol using dynamic resolution:
- **Major Quotes (priority order):** `usdt`, `usdc`, `usd`, `eur`, `gbp`, `jpy`, `cad`, `aud`, `chf`, `btc`, `eth`
- **Dynamic Resolution:** Asset IDs are resolved from the data files rather than hardcoded
- **Position Detection:** Check both start and end positions
- **Separator Handling:** Remove common separators (`/`, `-`, `_`, `.`)
- **High Confidence Matching:** Uses 95% confidence threshold for quote currency resolution

##### 3. Base Asset Resolution
**If quote detected:**
- Resolve remaining symbol within specified asset class
- If remaining symbol is empty (e.g., just "EUR"), use detected asset as base with USD quote

**If no quote detected:**
- Resolve entire cleaned symbol as single asset within specified asset class
- Use USD as default quote currency

##### 4. Ticker Construction
Create final `Ticker` object with:
- 64-bit ticker ID encoding asset classification
- Human-readable name (e.g., "EUR/USD")
- Base and quote `Asset` objects
- Processing steps for debugging

### Symbol Resolution Conventions

The BTR system standardizes instrument symbols by programmatically handling common platform-specific prefixes and suffixes rather than storing them as explicit aliases.

#### Automatic Ticker Prefix/Suffix Stripping Rules

**Common Prefixes:** `^`, `.`, `$`, `#` (e.g., `^SPX`, `.DJI`, `$INDU`, `#GOLD`)

**Suffix Categories:**
1. **Delimiter-Based Suffixes** (stripped when following `-`, `_`, `.`, `$`, `^`, `#`):
   - `US`, `M`, `C`, `Z`, `B`, `R`, `D`, `I` (case-insensitive)
   - Examples: `SPX.US`, `DJI_C`, `GOLD$m`, `NDQ-m`, `SILVER#C`...

2. **Standalone Suffixes** (stripped regardless of delimiters):
   - `USX`, `MINI`, `MICRO`, `CASH`, `SPOT`, `ECN`, `ZERO` (case-insensitive)
   - Examples: `SPXmini`, `DJIcash`, `GOLD.spot`, `SILVER_ecn`...

**Compound Suffix Handling:** The resolution logic runs **twice** to handle compound suffixes like `XAG.CASH`, `NDQ$MICRO`, or `GOLD#SPOT`, where the first pass removes the descriptive suffix and the second pass removes the delimiter.

**Stripped Symbol Convention:** Any stripped ticker symbol is considered USD denominated unless a quote currency is explicitly detected.

### Comprehensive Resolution Examples

| Input | Asset Class | Processing | Result |
|-------|-------------|------------|---------|
| `"EUR/USD"` | `FX` | Quote detected: USD at end → Resolve EUR in FX | `EUR/USD` spot forex |
| `"EURUSD"` | `FX` | Quote detected: USD at end → Resolve EUR in FX | `EUR/USD` spot forex |
| `"SPY.cash"` | `EQ` | Strip ".cash" → Resolve SPY in EQ → Default USD quote | `SPY/USD` spot equity |
| `"GBPJPYmini"` | `FX` | Strip "mini" → Quote detected: JPY at end → Resolve GBP | `GBP/JPY` spot forex |
| `"EUR"` | `FX` | Quote detected: EUR → Empty remaining → EUR base, USD quote | `EUR/USD` spot forex |
| `"GOLD"` | `CM` | No quote detected → Resolve GOLD in CM → Default USD quote | `GOLD/USD` spot commodity |

### Quote Currency Detection

The system scans for major quote currencies with priority ordering:

| Asset Class | Quote Priority |
|-------------|---------------|
| Forex | USD, EUR, GBP, JPY, CHF, CAD, AUD |
| Crypto | USDT, USDC, BTC, ETH, USD |
| Equities | USD (regional currencies contextually) |
| Commodities | USD (convention) |
| Indices | USD |

### Asset Resolution Algorithm

```rust
pub fn resolve_symbol_to_ticker(
    symbol: &str,
    asset_class: AssetClass,
    instrument_type: InstrumentType,
) -> Result<u64, ResolutionError> {
    // 1. Normalize and clean symbol
    let cleaned = normalize_and_clean_symbol(symbol);
    
    // 2. Detect quote currency/asset
    let (base_symbol, quote_asset) = detect_quote_currency(&cleaned, asset_class)?;
    
    // 3. Resolve base asset
    let base_asset = resolve_asset(&base_symbol, asset_class)?;
    
    // 4. Generate ticker ID
    generate_ticker_id(
        instrument_type,
        base_asset.class,
        base_asset.id,
        quote_asset.class,
        quote_asset.id,
        0, // Default sub-type
    )
}
```

## Part 4: Best Practices

### Performance Optimizations

#### Memory Layout
- **Packed Structs**: Use `#[repr(C, packed)]` for zero-copy serialization
- **Bit Operations**: Leverage bit shifting for fast encoding/decoding
- **Cache-Friendly**: Pre-index data in hashmaps for O(1) lookups
- **Memory Efficiency**: Shared lowercase string instances reduce memory footprint

#### Processing Efficiency
- **Universal Lowercase Processing**: ALL input converted to lowercase immediately for consistent O(1) lookups
- **No Case Differentiation**: "BTC", "btc", "Btc" all resolve to the same asset
- **Pre-built Indices**: Asset data indexed by class and symbol for fast resolution
- **Batch Processing**: Process multiple symbols simultaneously when possible
- **Cache-Friendly Access**: Optimized patterns for fuzzy matching when exact matches fail

#### Indexing Strategy
- **By ID**: Direct lookup by `(AssetClass, u16)` tuple for O(1) access
- **By Normalized Name/Alias**: Exact string matching using lowercase indices
- **By Asset Class**: Filtered candidate lists for scoped fuzzy search
- **Global Fallback**: Full dataset search when class filtering fails
- **Dynamic Resolution**: Major quote currencies resolved at runtime from data files for consistency

### Data Consistency Rules

1. **Primary Symbol**: The most commonly used trading symbol should be the first in the aliases list
2. **ISO Codes**: Currency ISO codes (USD, EUR, etc.) are stored as aliases in data files
3. **Platform Independence**: Common prefixes/suffixes are handled programmatically, not stored as aliases
4. **Pipe Separation**: Multiple aliases are separated by `|` character in CSV files
5. **Universal Lowercase Processing**: ALL symbol matching uses lowercase normalization regardless of asset class - no case differentiation anywhere in the system
6. **UTF-8 Encoding**: All data files use UTF-8 encoding for international symbol support

### Validation Guidelines

```rust
pub fn validate_ticker_id(ticker_id: u64) -> Result<(), ValidationError> {
    let (inst_type, base_class, base_id, quote_class, quote_id, sub_type) = 
        extract_ticker_components(ticker_id);
    
    // Check instrument type
    if inst_type > 0xC {
        return Err(ValidationError::InvalidInstrumentType(inst_type));
    }
    
    // Check asset classes
    if base_class > 0xD || quote_class > 0xD {
        return Err(ValidationError::InvalidAssetClass);
    }
    
    // Check sub-type fits in 20 bits
    if sub_type > 0xFFFFF {
        return Err(ValidationError::SubTypeOverflow);
    }
    
    // Prevent self-referencing pairs (usually invalid)
    if base_class == quote_class && base_id == quote_id {
        return Err(ValidationError::SelfReferencingPair);
    }
    
    Ok(())
}
```

### Integration Patterns

#### With Channel IDs
```rust
// Create channel ID for EUR/USD ticks from Interactive Brokers
let ticker_id = 0x03006F301CD00000u64;  // EUR/USD spot
let channel_id = Channel::generate(691, MessageType::Tick);  // IBKR + ticks

// Subscribe to specific ticker on specific channel
subscribe_to_ticker_on_channel(ticker_id, channel_id);
```

#### With External Reference Data
- **FIGI/ISIN Mapping**: Map external identifiers to MITCH IDs
- **ISO Code Support**: Use ISO 4217 for currencies, maintain as aliases
- **Platform Symbols**: Handle exchange-specific symbols transparently

## Implementation Reference

The complete Rust implementation of this ticker system can be found in:
- **Asset Resolution**: `mitch/impl/rust/src/common.rs`
- **Constants & IDs**: `mitch/impl/rust/src/constants.rs`
- **Test Coverage**: `mitch/impl/rust/tests/ticker_test.rs`
