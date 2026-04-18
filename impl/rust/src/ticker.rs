//! MITCH Ticker ID encoding and asset type definitions.
//!
//! This module defines the wire-format types and encoding/decoding for ticker IDs.
//! Resolution logic (fuzzy matching, symbol parsing) lives in nxr-sdk.

use crate::common::{MitchError, AssetClass, InstrumentType};
use core::fmt;

// =============================================================================
// CORE DATA STRUCTURES
// =============================================================================

/// Pack asset class and ID into 32-bit global identifier (as per asset.md)
pub fn pack_asset(asset_class: AssetClass, class_id: u16) -> u32 {
    ((asset_class as u32) << 16) | (class_id as u32)
}

/// Unpack global asset ID into class and class_id (as per asset.md)
pub fn unpack_asset(packed_asset: u32) -> (AssetClass, u16) {
    let class_id = ((packed_asset >> 16) & 0xF) as u8;
    let asset_id = (packed_asset & 0xFFFF) as u16;
    (AssetClass::from_id(class_id), asset_id)
}

/// Asset with pre-normalized search keys
#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    /// Global asset ID (32-bit: 4-bit class + 16-bit class_id, as per asset.md)
    pub id: u32,
    /// Asset ID within its class (16-bit)
    pub class_id: u16,
    /// Asset class (4-bit encoded)
    pub class: AssetClass,
    /// Human-readable name
    pub name: String,
    /// Pipe-separated aliases for resolution
    pub aliases: String,
}

/// Complete ticker representation
#[derive(Debug, Clone, PartialEq)]
pub struct Ticker {
    /// 64-bit ticker ID for MITCH protocol messages
    pub id: u64,
    /// Human-readable name/symbol
    pub name: String,
    /// Instrument type (spot, futures, etc.)
    pub instrument_type: InstrumentType,
    /// Base asset
    pub base: Asset,
    /// Quote asset
    pub quote: Asset,
    /// Sub-type specification (20-bit)
    pub sub_type: u32,
}

/// Asset search result with confidence score
#[derive(Debug, Clone, PartialEq)]
pub struct AssetMatch {
    pub asset: Asset,
    pub confidence: f64,
    pub matched_field: String,
}

/// Ticker resolution result
#[derive(Debug, Clone, PartialEq)]
pub struct TickerMatch {
    pub ticker: Ticker,
    pub confidence: f64,
    pub processing_steps: Vec<String>,
}

// =============================================================================
// TICKER ID ENCODING/DECODING (64-bit)
// =============================================================================

/// 64-bit ticker ID wire format.
///
/// ```text
/// Bits   | Field              | Size
/// -------|--------------------|------
/// 63-60  | Instrument Type    | 4
/// 59-56  | Base Asset Class   | 4
/// 55-40  | Base Asset ID      | 16
/// 39-36  | Quote Asset Class  | 4
/// 35-20  | Quote Asset ID     | 16
/// 19-0   | Sub-Type           | 20
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickerId {
    pub raw: u64,
}

impl TickerId {
    pub fn new(
        instrument_type: InstrumentType,
        base_class: AssetClass,
        base_id: u16,
        quote_class: AssetClass,
        quote_id: u16,
        sub_type: u32,
    ) -> Result<Self, MitchError> {
        if sub_type > 0xFFFFF {
            return Err(MitchError::InvalidData("Sub-type must fit in 20 bits".into()));
        }
        let raw = ((instrument_type as u64) << 60)
            | ((base_class as u64) << 56)
            | ((base_id as u64) << 40)
            | ((quote_class as u64) << 36)
            | ((quote_id as u64) << 20)
            | (sub_type as u64);
        Ok(Self { raw })
    }

    pub fn from_raw(raw: u64) -> Self { Self { raw } }

    pub fn instrument_type(&self) -> InstrumentType {
        InstrumentType::from_id(((self.raw >> 60) & 0x0F) as u8)
    }
    pub fn base_asset_class(&self) -> AssetClass {
        AssetClass::from_id(((self.raw >> 56) & 0x0F) as u8)
    }
    pub fn base_asset_id(&self) -> u16 { ((self.raw >> 40) & 0xFFFF) as u16 }
    pub fn quote_asset_class(&self) -> AssetClass {
        AssetClass::from_id(((self.raw >> 36) & 0x0F) as u8)
    }
    pub fn quote_asset_id(&self) -> u16 { ((self.raw >> 20) & 0xFFFF) as u16 }
    pub fn sub_type(&self) -> u32 { (self.raw & 0xFFFFF) as u32 }

    pub fn pack(&self) -> [u8; 8] { self.raw.to_le_bytes() }

    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < 8 {
            return Err(MitchError::BufferTooSmall { expected: 8, actual: bytes.len() });
        }
        unsafe {
            let raw = (bytes.as_ptr() as *const u64).read_unaligned().to_le();
            Ok(Self::from_raw(raw))
        }
    }

    pub fn is_forex(&self) -> bool {
        matches!(self.base_asset_class(), AssetClass::FX) ||
        matches!(self.quote_asset_class(), AssetClass::FX)
    }
    pub fn is_crypto(&self) -> bool {
        matches!(self.base_asset_class(), AssetClass::CR) ||
        matches!(self.quote_asset_class(), AssetClass::CR)
    }
    pub fn is_spot(&self) -> bool {
        matches!(self.instrument_type(), InstrumentType::SPOT)
    }
}

impl From<u64> for TickerId {
    fn from(raw: u64) -> Self { Self::from_raw(raw) }
}

impl From<TickerId> for u64 {
    fn from(ticker: TickerId) -> Self { ticker.raw }
}

impl fmt::Display for TickerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TickerId({:016X}: {:?} base={:?}:{} quote={:?}:{} sub={})",
            self.raw, self.instrument_type(),
            self.base_asset_class(), self.base_asset_id(),
            self.quote_asset_class(), self.quote_asset_id(),
            self.sub_type())
    }
}

// ---- Convenience constructors ----

pub fn forex_ticker(base_id: u16, quote_id: u16, instrument_type: InstrumentType, sub_type: u32) -> Result<TickerId, MitchError> {
    TickerId::new(instrument_type, AssetClass::FX, base_id, AssetClass::FX, quote_id, sub_type)
}

pub fn crypto_ticker(base_id: u16, quote_id: u16, instrument_type: InstrumentType, sub_type: u32) -> Result<TickerId, MitchError> {
    TickerId::new(instrument_type, AssetClass::CR, base_id, AssetClass::CR, quote_id, sub_type)
}

pub fn equity_ticker(equity_id: u16, quote_currency_id: u16, instrument_type: InstrumentType, sub_type: u32) -> Result<TickerId, MitchError> {
    TickerId::new(instrument_type, AssetClass::EQ, equity_id, AssetClass::FX, quote_currency_id, sub_type)
}

// ---- Batch operations ----

pub fn unpack_ticker_batch(buffer: &[u8], count: usize) -> Result<Vec<TickerId>, MitchError> {
    let expected = count * 8;
    if buffer.len() < expected {
        return Err(MitchError::BufferTooSmall { expected, actual: buffer.len() });
    }
    let mut tickers = Vec::with_capacity(count);
    unsafe {
        let ptr = buffer.as_ptr() as *const u64;
        for i in 0..count {
            tickers.push(TickerId::from_raw(ptr.add(i).read_unaligned().to_le()));
        }
    }
    Ok(tickers)
}

pub fn pack_ticker_batch(tickers: &[TickerId]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(tickers.len() * 8);
    unsafe {
        buffer.set_len(tickers.len() * 8);
        let mut ptr = buffer.as_mut_ptr() as *mut u64;
        for ticker in tickers {
            ptr.write_unaligned(ticker.raw.to_le());
            ptr = ptr.add(1);
        }
    }
    buffer
}
