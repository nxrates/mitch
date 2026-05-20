//! OrderBook Message Implementation (2072 bytes)
//!
//! OrderBook messages provide complete market depth snapshots with aggregated
//! price levels in adaptive bins. Optimized for real-time order book state
//! distribution with minimal bandwidth overhead.
//!
//! # Message Layout (2072 bytes total)
//!
//! ```text
//! Offset | Field             | Type      | Size | Description
//! -------|-------------------|-----------|------|---------------------------
//! 0      | ticker            | u64       | 8    | Ticker identifier
//! 8      | mid_price         | f64       | 8    | Calculated mid price
//! 16     | bin_aggregator    | u8        | 1    | Aggregation function used
//! 17     | _pad              | [u8; 7]   | 7    | Padding for alignment
//! 24     | bids              | [Bin; 128]| 1024 | Bid levels (aggregated)
//! 1048   | asks              | [Bin; 128]| 1024 | Ask levels (aggregated)
//! ```

use crate::body::MitchBody;
use crate::common::{message_sizes, MitchError, BinAggregator};

/// Aggregated price-level bin (8 bytes).
///
/// ## Wire layout (little-endian)
///
/// ```text
/// Offset | Field       | Size | Type   | Description
/// -------|-------------|------|--------|------------------------------------
/// 0      | order_count | 4    | u32 LE | Number of orders at this level
/// 4      | volume      | 4    | u32 LE | Aggregated volume at this level
/// ```
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Bin {
    pub order_count: u32,
    pub volume: u32,
}

impl Bin {
    /// Create a new bin
    pub fn new(order_count: u32, volume: u32) -> Self {
        Self { order_count, volume }
    }

    /// Check if bin is empty
    pub fn is_empty(&self) -> bool {
        self.volume == 0
    }
}

/// OrderBook message structure (2072 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct OrderBook {
    /// Ticker identifier (8 bytes)
    pub ticker: u64,
    /// Mid price (8 bytes)
    pub mid_price: f64,
    /// Bin aggregator ID (1 byte)
    pub bin_aggregator: u8,
    /// Padding (7 bytes)
    pub _pad: [u8; 7],
    /// Bid bins (1024 bytes)
    pub bids: [Bin; 128],
    /// Ask bins (1024 bytes)
    pub asks: [Bin; 128],
}

impl OrderBook {
    /// Create a new OrderBook message
    ///
    /// # Arguments
    /// * `ticker` - Ticker identifier
    /// * `mid_price` - Calculated mid price
    /// * `bin_aggregator` - Aggregation function ID (0-3)
    /// * `bids` - Array of 128 bid bins
    /// * `asks` - Array of 128 ask bins
    pub fn new(
        ticker: u64,
        mid_price: f64,
        bin_aggregator: u8,
        bids: [Bin; 128],
        asks: [Bin; 128],
    ) -> Self {
        Self {
            ticker,
            mid_price,
            bin_aggregator,
            _pad: [0; 7],
            bids,
            asks,
        }
    }

    /// Pack to bytes using zero-copy transmutation
    pub fn pack(&self) -> [u8; message_sizes::ORDER_BOOK] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Unpack from bytes using zero-copy read
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::ORDER_BOOK {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::ORDER_BOOK,
                actual: bytes.len(),
            });
        }
        unsafe {
            let ptr = bytes.as_ptr() as *const Self;
            Ok(ptr.read_unaligned())
        }
    }

    /// Unpack without bounds check (unsafe, max performance)
    pub unsafe fn unpack_unchecked(bytes: &[u8]) -> Self {
        let ptr = bytes.as_ptr() as *const Self;
        ptr.read_unaligned()
    }

    /// Calculate total bid volume
    pub fn total_bid_volume(&self) -> u64 {
        let bids = unsafe { std::ptr::addr_of!(self.bids).read_unaligned() };
        bids.iter().map(|bin| bin.volume as u64).sum()
    }

    /// Calculate total ask volume
    pub fn total_ask_volume(&self) -> u64 {
        let asks = unsafe { std::ptr::addr_of!(self.asks).read_unaligned() };
        asks.iter().map(|bin| bin.volume as u64).sum()
    }

    /// Get bin aggregator enum
    pub fn aggregator_type(&self) -> BinAggregator {
        match self.bin_aggregator {
            0 => BinAggregator::DEFAULT_LINGAUSSIAN,
            1 => BinAggregator::DEFAULT_LINGEOFLAT,
            2 => BinAggregator::DEFAULT_BILINGEO,
            3 => BinAggregator::DEFAULT_TRILINEAR,
            _ => BinAggregator::DEFAULT_LINGAUSSIAN,
        }
    }

    /// Validate message integrity
    pub fn validate(&self) -> Result<(), MitchError> {
        if self.mid_price <= 0.0 {
            return Err(MitchError::InvalidFieldValue("mid_price".into()));
        }
        if self.bin_aggregator > 3 {
            return Err(MitchError::InvalidFieldValue("bin_aggregator".into()));
        }
        Ok(())
    }

    /// Get the size of the OrderBook struct in bytes.
    pub const fn size() -> usize {
        message_sizes::ORDER_BOOK
    }
}

impl core::fmt::Display for OrderBook {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let ticker = unsafe { std::ptr::addr_of!(self.ticker).read_unaligned() };
        let mid_price = unsafe { std::ptr::addr_of!(self.mid_price).read_unaligned() };
        let bin_aggregator = unsafe { std::ptr::addr_of!(self.bin_aggregator).read_unaligned() };
        write!(
            f,
            "OrderBook(ticker={:016X}, mid={:.6}, aggregator={}, bid_vol={}, ask_vol={})",
            ticker,
            mid_price,
            bin_aggregator,
            self.total_bid_volume(),
            self.total_ask_volume()
        )
    }
}

// SAFETY: OrderBook is `#[repr(C, packed)]` with only POD fields; no padding bytes.
unsafe impl MitchBody for OrderBook {
    const SIZE: usize = message_sizes::ORDER_BOOK;
}

// SAFETY: Bin is `#[repr(C, packed)]` with only POD fields; no padding bytes.
unsafe impl MitchBody for Bin {
    const SIZE: usize = message_sizes::BIN;
}

// Compile-time size assertions
const _: () = assert!(core::mem::size_of::<Bin>() == message_sizes::BIN, "Bin must be exactly 8 bytes");
const _: () = assert!(core::mem::size_of::<OrderBook>() == message_sizes::ORDER_BOOK, "OrderBook must be exactly 2072 bytes");
