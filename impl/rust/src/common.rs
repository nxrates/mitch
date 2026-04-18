//! Common types, enums, and constants used across MITCH protocol messages
//!
//! This module defines the foundational types that are shared between different
//! message types in the MITCH protocol, ensuring consistency and type safety.

use core::fmt;

// =============================================================================
// MESSAGE TYPE CONSTANTS
// =============================================================================

/// MITCH message type codes (ASCII)
pub mod message_type {
    /// Trade message ('t')
    pub const TRADE: u8 = b't';        // 116
    /// Order message ('o')
    pub const ORDER: u8 = b'o';        // 111
    /// Tick message ('s')
    pub const TICK: u8 = b's';         // 115
    /// Index message ('i')
    pub const INDEX: u8 = b'i';        // 105
    /// Order book message ('b')
    pub const ORDER_BOOK: u8 = b'b';   // 98
    /// Bar / kline ('k')
    pub const BAR: u8 = b'k';         // 107
}

/// Message size constants in bytes
pub mod message_sizes {
    /// Asset size
    pub const ASSET: usize = 20;
    /// Ticker size
    pub const TICKER: usize = 64;
    /// Header size (16 bytes)
    pub const HEADER: usize = 16;
    /// Trade body size
    pub const TRADE: usize = 24;
    /// Order body size
    pub const ORDER: usize = 32;
    /// Tick body size
    pub const TICK: usize = 32;
    /// Index body size (40B body = 56B frame with 16B header)
    pub const INDEX: usize = 40;
    /// Order book body size
    pub const ORDER_BOOK: usize = 2072;
    /// Bin size
    pub const BIN: usize = 8;
    /// Bar body size (2 cache lines)
    pub const BAR: usize = 128;
}

// =============================================================================
// TRADING ENUMS
// =============================================================================

/// Order side enumeration (buy/sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OrderSide {
    /// Buy side (0)
    Buy = 0,
    /// Sell side (1)
    Sell = 1,
}

// Safety: OrderSide is #[repr(u8)] and all bit patterns 0..=1 are valid variants.
// Pod requires all bit patterns to be valid, but u8 has 256 patterns and only 2 are valid.
// We use Zeroable (0 = Buy is valid) but NOT Pod. Trade uses a manual Pod impl instead.
#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Zeroable for OrderSide {}

impl Default for OrderSide {
    fn default() -> Self {
        OrderSide::Buy
    }
}

/// Order type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OrderType {
    /// Market order (0)
    Market = 0,
    /// Limit order (1)
    Limit = 1,
    /// Stop order (2)
    Stop = 2,
    /// Cancel order (3)
    Cancel = 3,
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::Market
    }
}

// =============================================================================
// ASSET CLASSIFICATION
// =============================================================================

/// Re-export types from constants module
pub use crate::constants::{AssetClass, InstrumentType, BinAggregator};

// =============================================================================
// ERROR HANDLING
// =============================================================================

/// Custom error type for MITCH protocol operations.
#[derive(Debug, PartialEq)]
pub enum MitchError {
    /// Error representing invalid or corrupt data.
    InvalidData(String),
    /// Error for an unrecognized message type.
    InvalidMessageType(u8),
    /// Error indicating a buffer is too small for a message.
    BufferTooSmall {
        /// The expected size of the buffer.
        expected: usize,
        /// The actual size of the buffer.
        actual: usize,
    },
    /// Error for an invalid ticker ID.
    InvalidTickerId(String),
    /// Error for an invalid channel ID.
    InvalidChannelId(String),
    /// Error for a field containing an invalid value.
    InvalidFieldValue(String),
    /// Error during serialization.
    SerializationError(String),
}

impl fmt::Display for MitchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MitchError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            MitchError::InvalidMessageType(t) => write!(f, "Invalid message type: {}", t),
            MitchError::BufferTooSmall { expected, actual } => {
                write!(f, "Buffer too small: expected {}, got {}", expected, actual)
            }
            MitchError::InvalidTickerId(msg) => write!(f, "Invalid ticker ID: {}", msg),
            MitchError::InvalidChannelId(msg) => write!(f, "Invalid channel ID: {}", msg),
            MitchError::InvalidFieldValue(msg) => write!(f, "Invalid field value: {}", msg),
            MitchError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MitchError {}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Extract the side from a combined type_and_side field (bit 0)
pub fn extract_order_side(type_and_side: u8) -> OrderSide {
    match type_and_side & 0x01 {
        0 => OrderSide::Buy,
        _ => OrderSide::Sell,
    }
}

/// Extract the order type from a combined type_and_side field (bits 1-7)
pub fn extract_order_type(type_and_side: u8) -> OrderType {
    match (type_and_side >> 1) & 0x7F {
        0 => OrderType::Market,
        1 => OrderType::Limit,
        2 => OrderType::Stop,
        3 => OrderType::Cancel,
        _ => OrderType::Market, // Default fallback
    }
}

/// Combine order type and side into a single field
pub fn combine_type_and_side(order_type: OrderType, side: OrderSide) -> u8 {
    ((order_type as u8) << 1) | (side as u8)
}

/// Get the ASCII character for a message type code
pub fn message_type_char(msg_type: u8) -> Option<char> {
    match msg_type {
        message_type::TRADE => Some('t'),
        message_type::ORDER => Some('o'),
        message_type::TICK => Some('s'),
        message_type::INDEX => Some('i'),
        message_type::ORDER_BOOK => Some('b'),
        message_type::BAR => Some('k'),
        _ => None,
    }
}

/// Validate that a message type is supported
pub fn validate_message_type(msg_type: u8) -> Result<(), MitchError> {
    match message_type_char(msg_type) {
        Some(_) => Ok(()),
        None => Err(MitchError::InvalidMessageType(msg_type)),
    }
}

// =============================================================================
// MESSAGE TYPE ↔ 4-BIT CODE MAPPING (for MitchHeader type_provider field)
// =============================================================================

/// 4-bit wire codes for message types (stored in low nibble of type_provider).
pub mod message_type_code {
    pub const TRADE: u8 = 1;
    pub const ORDER: u8 = 2;
    pub const TICK: u8 = 3;
    pub const INDEX: u8 = 4;
    pub const ORDER_BOOK: u8 = 5;
    pub const BAR: u8 = 6;
}

/// Map ASCII message type to 4-bit wire code. Returns 0 on invalid input.
#[inline]
pub fn msg_type_to_code(ascii: u8) -> u8 {
    match ascii {
        message_type::TRADE => message_type_code::TRADE,
        message_type::ORDER => message_type_code::ORDER,
        message_type::TICK => message_type_code::TICK,
        message_type::INDEX => message_type_code::INDEX,
        message_type::ORDER_BOOK => message_type_code::ORDER_BOOK,
        message_type::BAR => message_type_code::BAR,
        _ => 0,
    }
}

/// Map 4-bit wire code back to ASCII message type. Returns 0 on invalid input.
#[inline]
pub fn code_to_msg_type(code: u8) -> u8 {
    match code {
        message_type_code::TRADE => message_type::TRADE,
        message_type_code::ORDER => message_type::ORDER,
        message_type_code::TICK => message_type::TICK,
        message_type_code::INDEX => message_type::INDEX,
        message_type_code::ORDER_BOOK => message_type::ORDER_BOOK,
        message_type_code::BAR => message_type::BAR,
        _ => 0,
    }
}

/// Validate a 4-bit wire code is a known message type.
#[inline]
pub fn validate_message_type_code(code: u8) -> Result<(), MitchError> {
    if code >= 1 && code <= 6 {
        Ok(())
    } else {
        Err(MitchError::InvalidMessageType(code))
    }
}

