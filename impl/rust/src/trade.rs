//! MITCH Trade message implementation
//!
//! Trade messages (`t`) represent executed transactions in a market,
//! capturing price, volume, participant, and timing information.

use crate::body::MitchBody;
use crate::common::{message_sizes, OrderSide, MitchError};

/// Trade execution data (24 bytes)
///
/// Structure per trade.md specification:
/// | Field    | Offset | Size | Type     | Description                            |
/// |----------|--------|------|----------|----------------------------------------|
/// | Ticker   | 0      | 8    | `u64`    | 8-byte ticker identifier               |
/// | Price    | 8      | 8    | `f64`    | Execution price                        |
/// | Qty      | 16     | 4    | `u32`    | Executed volume/quantity               |
/// | Trade ID | 20     | 3    | `[u8;3]` | Unique trade identifier (u24 LE)       |
/// | Side     | 23     | 1    | `u8`     | 0: Buy, 1: Sell                       |
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Trade {
    /// Ticker identifier (8 bytes)
    pub ticker: u64,
    /// Execution price (8 bytes)
    pub price: f64,
    /// Executed volume (4 bytes)
    pub qty: u32,
    /// Unique trade identifier stored as u24 little-endian (3 bytes, max 16,777,215)
    pub trade_id: [u8; 3],
    /// Trade side (1 byte: 0=Buy, 1=Sell)
    pub side: u8,
}

impl Trade {
    /// Create a new Trade message with validation.
    ///
    /// `trade_id` is accepted as a `u32` for convenience and encoded
    /// internally as a 3-byte little-endian value (max 16,777,215).
    pub fn new(
        ticker: u64,
        price: f64,
        qty: u32,
        trade_id: u32,
        side: OrderSide,
    ) -> Result<Self, MitchError> {
        let mut trade = Self {
            ticker,
            price,
            qty,
            trade_id: [0; 3],
            side: side as u8,
        };
        trade.set_trade_id(trade_id);
        trade.validate()?;
        Ok(trade)
    }

    /// Read the 3-byte trade_id as a u32 (little-endian u24).
    pub fn get_trade_id(&self) -> u32 {
        let b = self.trade_id;
        b[0] as u32 | (b[1] as u32) << 8 | (b[2] as u32) << 16
    }

    /// Write a u32 value into the 3-byte trade_id field (little-endian u24).
    ///
    /// Only the low 24 bits are stored; the top byte is silently discarded.
    pub fn set_trade_id(&mut self, id: u32) {
        self.trade_id = [
            id as u8,
            (id >> 8) as u8,
            (id >> 16) as u8,
        ];
    }

    /// Pack Trade into bytes using zero-copy transmutation.
    pub fn pack(&self) -> [u8; message_sizes::TRADE] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Unpack Trade from bytes using a zero-copy read.
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::TRADE {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::TRADE,
                actual: bytes.len(),
            });
        }
        let trade = unsafe { (bytes.as_ptr() as *const Self).read_unaligned() };
        trade.validate()?;
        Ok(trade)
    }

    /// Validate the contents of the Trade message.
    pub fn validate(&self) -> Result<(), MitchError> {
        if self.ticker == 0 {
            return Err(MitchError::InvalidFieldValue("ticker cannot be zero".into()));
        }
        if self.price <= 0.0 {
            return Err(MitchError::InvalidFieldValue("price must be positive".into()));
        }
        if self.qty == 0 {
            return Err(MitchError::InvalidFieldValue("qty must be positive".into()));
        }
        if self.get_trade_id() == 0 {
            return Err(MitchError::InvalidFieldValue("trade_id cannot be zero".into()));
        }
        Ok(())
    }

    /// Get the size of the Trade struct in bytes.
    pub const fn size() -> usize {
        message_sizes::TRADE
    }

    /// Get the trade side as an OrderSide enum.
    pub fn get_side(&self) -> OrderSide {
        if self.side == 1 { OrderSide::Sell } else { OrderSide::Buy }
    }

    /// Get notional value (price * quantity).
    pub fn notional_value(&self) -> f64 {
        self.price * self.qty as f64
    }

    /// Check if this is a buy trade.
    pub fn is_buy(&self) -> bool {
        self.side == 0
    }

    /// Check if this is a sell trade.
    pub fn is_sell(&self) -> bool {
        self.side == 1
    }
}

impl Default for Trade {
    fn default() -> Self {
        Self {
            ticker: 0,
            price: 0.0,
            qty: 0,
            trade_id: [0; 3],
            side: 0,
        }
    }
}

// =============================================================================
// DISPLAY IMPLEMENTATIONS
// =============================================================================

impl core::fmt::Display for Trade {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let ticker = self.ticker;
        let price = self.price;
        let qty = self.qty;
        let trade_id = self.get_trade_id();
        let side = self.get_side();

        write!(
            f,
            "Trade {{ ticker: {:#X}, price: {}, qty: {}, id: {}, side: {:?} }}",
            ticker, price, qty, trade_id, side
        )
    }
}

// SAFETY: Trade is `#[repr(C, packed)]` with only POD fields; no padding bytes.
unsafe impl MitchBody for Trade {
    const SIZE: usize = message_sizes::TRADE;
}

// Compile-time size assertion
const _: () = assert!(core::mem::size_of::<Trade>() == message_sizes::TRADE, "Trade must be exactly 24 bytes");
