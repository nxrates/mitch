//! MITCH Tick message implementation
//!
//! Tick messages (`s`) provide point-in-time bid/ask ticker snapshots representing
//! the best prices and activity in a market. They capture level-1 (top of book) data.

use crate::body::MitchBody;
use crate::common::{message_sizes, MitchError};

/// Tick / quote snapshot (32 bytes).
///
/// ## Wire layout (little-endian)
///
/// ```text
/// Offset | Field   | Size | Type   | Description
/// -------|---------|------|--------|------------------------------------------
/// 0      | ticker  | 8    | u64 LE | Ticker identifier
/// 8      | bid     | 8    | f64 LE | Best bid price
/// 16     | ask     | 8    | f64 LE | Best ask price
/// 24     | vbid    | 4    | u32 LE | Aggregated bid volume
/// 28     | vask    | 4    | u32 LE | Aggregated ask volume
/// ```
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Tick {
    pub ticker: u64,
    pub bid: f64,
    pub ask: f64,
    pub vbid: u32,
    pub vask: u32,
}

impl Tick {
    /// Create a new Tick message with validation.
    pub fn new(
        ticker: u64,
        bid: f64,
        ask: f64,
        vbid: u32,
        vask: u32,
    ) -> Result<Self, MitchError> {
        let tick = Self {
            ticker,
            bid,
            ask,
            vbid,
            vask,
        };
        tick.validate()?;
        Ok(tick)
    }

    /// Create a new Tick without validation (use with trusted data, e.g. from wire format)
    #[inline]
    pub const fn new_unchecked(
        ticker: u64,
        bid: f64,
        ask: f64,
        vbid: u32,
        vask: u32,
    ) -> Self {
        Self { ticker, bid, ask, vbid, vask }
    }

    /// Pack Tick into bytes using zero-copy transmutation.
    pub fn pack(&self) -> [u8; message_sizes::TICK] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Unpack Tick from bytes using a zero-copy read.
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::TICK {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::TICK,
                actual: bytes.len(),
            });
        }
        let tick = unsafe { (bytes.as_ptr() as *const Self).read_unaligned() };
        tick.validate()?;
        Ok(tick)
    }

    /// Validate the contents of the Tick message.
    pub fn validate(&self) -> Result<(), MitchError> {
        if self.ticker == 0 {
            return Err(MitchError::InvalidFieldValue("ticker cannot be zero".into()));
        }
        if self.bid <= 0.0 {
            return Err(MitchError::InvalidFieldValue("bid must be positive".into()));
        }
        if self.ask <= 0.0 {
            return Err(MitchError::InvalidFieldValue("ask must be positive".into()));
        }
        if self.ask < self.bid {
            return Err(MitchError::InvalidFieldValue("ask cannot be less than bid".into()));
        }
        Ok(())
    }

    /// Calculate mid price.
    pub fn mid_price(&self) -> f64 {
        (self.bid + self.ask) / 2.0
    }

    /// Calculate spread.
    pub fn spread(&self) -> f64 {
        self.ask - self.bid
    }

    /// Calculate spread in basis points.
    pub fn spread_bps(&self) -> f64 {
        let mid = self.mid_price();
        if mid > 0.0 {
            (self.spread() / mid) * 10000.0
        } else {
            0.0
        }
    }

    /// Calculate total volume (vbid + vask).
    pub fn total_volume(&self) -> u64 {
        self.vbid as u64 + self.vask as u64
    }

    /// Calculate volume imbalance.
    /// Returns a value between -1.0 (all ask volume) and 1.0 (all bid volume).
    pub fn volume_imbalance(&self) -> f64 {
        let total = self.total_volume() as f64;
        if total > 0.0 {
            (self.vask as f64 - self.vbid as f64) / total
        } else {
            0.0
        }
    }

    /// Get the size of the Tick struct in bytes.
    pub const fn size() -> usize {
        message_sizes::TICK
    }
}

// SAFETY: Tick is `#[repr(C, packed)]` with only POD fields; no padding bytes.
unsafe impl MitchBody for Tick {
    const SIZE: usize = message_sizes::TICK;
}

// Compile-time size assertion
const _: () = assert!(core::mem::size_of::<Tick>() == message_sizes::TICK, "Tick must be exactly 32 bytes");
