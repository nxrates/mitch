//! MITCH (Moded Individual Trade Clearing and Handling) Protocol
//!
//! A transport-agnostic binary protocol for ultra-low latency market data.
//! See the [protocol overview](https://github.com/btr-trading/mitch/blob/main/model/overview.md) for more details.
//!
//! # Features
//! - `std`: (Default) Enables standard library features.
//! - `no_std`: For embedded and resource-constrained environments.
//! - `ffi`: Enables C FFI exports.
//! - `bytemuck`: Enables zero-copy Pod/Zeroable derives.
//!
//! # Quick Start
//!
//! ```rust
//! use mitch::{Trade, OrderSide};
//!
//! // Create a new trade message
//! let trade = Trade::new(12345, 99.95, 1000, 42, OrderSide::Buy).unwrap();
//! assert_eq!(std::mem::size_of_val(&trade), 24);
//!
//! // Create an index snapshot
//! let idx = mitch::Index::new(1, 99.9, 100.1, 50, 500, 500, 15, 9, 5, 0);
//! assert_eq!(std::mem::size_of_val(&idx), 40);
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
// #![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![allow(clippy::all)]

// =============================================================================
// MODULE DECLARATIONS
// =============================================================================

/// Ticker ID and channel implementation
pub mod channel;
/// Generic zero-copy batch helpers + MitchBody trait
pub mod body;
/// Common types, enums, and constants used across all message types
pub mod common;
/// Generated constants from CSV files.
pub mod constants;
/// MITCH unified message header (16 bytes)
pub mod header;
/// Index message implementation (40 bytes)
pub mod index;
/// Market provider type definitions
pub mod market_providers;
/// Order message implementation (32 bytes)
pub mod order;
/// OrderBook message implementation (2072 bytes)
pub mod order_book;
/// Tick message implementation (32 bytes)
pub mod tick;
/// Bar - enriched OHLCV bar, kline or renko (96 bytes)
pub mod bar;
/// Heartbeat message implementation (16 bytes)
pub mod heartbeat;
/// Frame types - MitchHeader + body composition for wire/file I/O
pub mod frame;
/// u48 timestamp encoding: 16µs ticks since 2010-01-01
pub mod timestamp;
/// Ticker ID encoding/decoding and asset type definitions
pub mod ticker;
/// Trade message implementation (24 bytes)
pub mod trade;

// FFI: removed from mitch. Resolution logic now lives in nxr-sdk.
// C ABI will be rebuilt in nxr-forwarder (task #67).

// Re-export public API
pub use crate::body::*;
pub use crate::common::*;
pub use crate::header::*;
pub use crate::trade::*;
pub use crate::order::*;
pub use crate::tick::*;
pub use crate::bar::*;
pub use crate::heartbeat::*;
pub use crate::frame::*;
pub use crate::index::*;
pub use crate::order_book::*;
pub use crate::ticker::*;
pub use crate::channel::*;
pub use crate::market_providers::{MarketProvider, ProviderMatch};

// =============================================================================
// LIBRARY VERSION AND METADATA
// =============================================================================

/// MITCH protocol version implemented by this crate
pub const MITCH_VERSION: &str = "1.0.0";

/// Library version
pub const LIB_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build information
pub const BUILD_INFO: &str = concat!(
    "mitch-rust v", env!("CARGO_PKG_VERSION"),
    " (MITCH protocol v1.0.0)"
);

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/// Calculate message size for given type and count
///
/// # Arguments
/// * `message_type` - ASCII message type code
/// * `count` - Number of message bodies
///
/// # Returns
/// Total message size in bytes (header + bodies) or error
///
/// # Example
/// ```rust
/// use mitch::*;
///
/// // Size for 10 trade messages
/// let size = calculate_message_size(message_type::TRADE, 10).unwrap();
/// assert_eq!(size, 16 + (10 * 24)); // 16-byte header + 10 * 24-byte trades
/// ```
pub fn calculate_message_size(message_type: u8, count: u8) -> Result<usize, MitchError> {
    validate_message_type(message_type)?;

    let single_body_size = match message_type {
        message_type::TRADE => message_sizes::TRADE,
        message_type::ORDER => message_sizes::ORDER,
        message_type::TICK => message_sizes::TICK,
        message_type::INDEX => message_sizes::INDEX,
        message_type::ORDER_BOOK => message_sizes::ORDER_BOOK,
        message_type::BAR => message_sizes::BAR,
        message_type::HEARTBEAT => message_sizes::HEARTBEAT,
        _ => return Err(MitchError::InvalidMessageType(message_type)),
    };

    Ok(message_sizes::HEADER + (count as usize * single_body_size))
}

/// Validate that a buffer contains a valid MITCH message
///
/// # Arguments
/// * `bytes` - Buffer to validate
///
/// # Returns
/// Result containing message type (ASCII) and count, or error
///
/// # Example
/// ```rust
/// use mitch::*;
///
/// let trade = Trade::new(0x1, 100.0, 1000, 1, OrderSide::Buy).unwrap();
/// let header = MitchHeader::new(message_type::TRADE, 0, 123456, 1);
///
/// let mut buffer = Vec::new();
/// buffer.extend_from_slice(&header.pack());
/// buffer.extend_from_slice(&trade.pack());
///
/// let (msg_type, count) = validate_message_buffer(&buffer).unwrap();
/// assert_eq!(msg_type, message_type::TRADE);
/// assert_eq!(count, 1);
/// ```
pub fn validate_message_buffer(bytes: &[u8]) -> Result<(u8, u8), MitchError> {
    if bytes.len() < message_sizes::HEADER {
        return Err(MitchError::BufferTooSmall {
            expected: message_sizes::HEADER,
            actual: bytes.len(),
        });
    }

    let header = MitchHeader::unpack(bytes)?;
    let expected_size = calculate_message_size(header.message_type(), header.count)?;

    if bytes.len() < expected_size {
        return Err(MitchError::BufferTooSmall {
            expected: expected_size,
            actual: bytes.len(),
        });
    }

    Ok((header.message_type(), header.count))
}
