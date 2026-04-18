//! Index Message Implementation (40 bytes)
//!
//! Unified aggregated market data type. Produced by the NX Rates aggregator
//! with VWAP bid/ask, confidence interval, and quality metrics.
//!
//! `mid` is NOT stored - it is always `(bid + ask) / 2` and derived via `mid()`.
//! `spread` is NOT stored - it is always `ask - bid` and derived via `spread()`.
//! Timestamps live in the 8-byte MitchHeader (same as Tick/Trade types).
//!
//! # Message Layout (40 bytes body, 56 bytes with 16B header)
//!
//! ```text
//! Offset | Field      | Size | Type  | Description
//! -------|------------|------|-------|------------------------------------
//! 0      | ticker     | 8    | u64   | MITCH ticker identifier
//! 8      | bid        | 8    | f64   | Best bid price (VWAP composite)
//! 16     | ask        | 8    | f64   | Best ask price (VWAP composite)
//! 24     | vbid       | 4    | u32   | Aggregated bid volume
//! 28     | vask       | 4    | u32   | Aggregated ask volume
//! 32     | ci         | 2    | u16   | Confidence interval in UBP
//! 34     | tick_count | 2    | u16   | Raw ticks in aggregation window
//! 36     | confidence | 1    | u8    | Active provider count
//! 37     | accepted   | 1    | u8    | Accepted providers
//! 38     | rejected   | 1    | u8    | Rejected providers
//! 39     | _pad       | 1    | [u8;1]| Reserved padding
//! ```

use crate::body::MitchBody;
use crate::common::{message_sizes, MitchError};
use core::fmt;

/// Index message structure (40 bytes)
///
/// Unified aggregated market data. `mid` is derived: `(bid + ask) / 2`.
/// Timestamps live in the MitchHeader, not in this body struct.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Index {
    /// Ticker identifier (8 bytes)
    pub ticker: u64,
    /// Best bid price - VWAP composite (8 bytes)
    pub bid: f64,
    /// Best ask price - VWAP composite (8 bytes)
    pub ask: f64,
    /// Aggregated bid volume (4 bytes)
    pub vbid: u32,
    /// Aggregated ask volume (4 bytes)
    pub vask: u32,
    /// Confidence interval - sqrt-compressed micro basis points of mid (2 bytes).
    ///
    /// # Encoding
    ///
    /// ```text
    /// encoded (u16) = round( sqrt(ci_ubp) * CI_SCALE )    // CI_SCALE = 16.0
    /// ci_ubp (f64)  = (encoded / CI_SCALE) ^ 2            // inverse
    /// ci_price      = mid * ci_ubp / 1e8                  // price-space interval
    /// ```
    ///
    /// `ci_ubp` is the 1-sigma confidence interval expressed in micro basis
    /// points of mid (1 ubp = 1e-8 x mid). The sqrt compression gives a
    /// dynamic range of roughly `[0, 16.77e6]` ubp (~16.77% of mid) before
    /// u16 saturation, versus the old flat linear encoding which saturated
    /// at 65535 ubp (~0.065% of mid).
    ///
    /// The reference encode / decode helpers live in
    /// `nxr_sdk::tdwap::{encode_ci_ubp, decode_ci_ubp}`.
    /// `Self::ci_price()` applies the inverse for you.
    pub ci: u16,
    /// Raw ticks in aggregation window (2 bytes)
    pub tick_count: u16,
    /// Active provider count (1 byte)
    pub confidence: u8,
    /// Accepted providers (1 byte)
    pub accepted: u8,
    /// Rejected providers (1 byte)
    pub rejected: u8,
    /// Reserved padding (1 byte)
    pub _pad: [u8; 1],
}

// Compile-time size assertion
const _: () = assert!(core::mem::size_of::<Index>() == 40, "Index must be exactly 40 bytes");

impl Index {
    /// Create a new Index message.
    pub fn new(
        ticker: u64,
        bid: f64,
        ask: f64,
        ci: u16,
        vbid: u32,
        vask: u32,
        tick_count: u16,
        confidence: u8,
        accepted: u8,
        rejected: u8,
    ) -> Self {
        Self {
            ticker,
            bid,
            ask,
            vbid,
            vask,
            ci,
            tick_count,
            confidence,
            accepted,
            rejected,
            _pad: [0; 1],
        }
    }

    // ── Serialization ──────────────────────────────────────────────────

    /// Pack Index message to bytes
    pub fn pack(&self) -> [u8; message_sizes::INDEX] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Unpack Index message from bytes
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::INDEX {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::INDEX,
                actual: bytes.len(),
            });
        }
        unsafe {
            let ptr = bytes.as_ptr() as *const Self;
            Ok(ptr.read_unaligned())
        }
    }

    /// Unpack without bounds checking (maximum performance)
    pub unsafe fn unpack_unchecked(bytes: &[u8]) -> Self {
        let ptr = bytes.as_ptr() as *const Self;
        ptr.read_unaligned()
    }

    // ── Derived calculations ───────────────────────────────────────────

    /// Mid price: (bid + ask) / 2. NOT stored - always derived.
    #[inline]
    pub fn mid(&self) -> f64 {
        (self.bid + self.ask) / 2.0
    }

    /// Decode CI from the sqrt-compressed wire format and convert to price units.
    ///
    /// Inverse of the encoding described on [`Self::ci`]:
    ///   `ci_ubp = (ci / 16)^2`, `ci_price = mid * ci_ubp / 1e8`.
    pub fn ci_price(&self) -> f64 {
        const CI_SCALE: f64 = 16.0;
        let x = self.ci as f64 / CI_SCALE;
        let ci_ubp = x * x;
        self.mid() * ci_ubp / 1e8
    }

    /// Spread: ask - bid. NOT stored - always derived.
    pub fn spread(&self) -> f64 {
        self.ask - self.bid
    }

    /// Spread in basis points: (ask - bid) / mid * 10000
    pub fn spread_bps(&self) -> f64 {
        (self.ask - self.bid) / self.mid() * 10000.0
    }

    /// Volume imbalance: (vask - vbid) / (vask + vbid)
    pub fn volume_imbalance(&self) -> f64 {
        let total = self.vask as f64 + self.vbid as f64;
        if total == 0.0 { return 0.0; }
        (self.vask as f64 - self.vbid as f64) / total
    }

    /// Get the size of the Index struct in bytes.
    pub const fn size() -> usize {
        message_sizes::INDEX
    }

    /// Validate message data integrity
    pub fn validate(&self) -> Result<(), MitchError> {
        if self.ticker == 0 { return Err(MitchError::InvalidFieldValue("Ticker cannot be zero".into())); }
        if self.bid <= 0.0 { return Err(MitchError::InvalidFieldValue("Bid price must be positive".into())); }
        if self.ask <= 0.0 { return Err(MitchError::InvalidFieldValue("Ask price must be positive".into())); }
        if self.ask < self.bid { return Err(MitchError::InvalidFieldValue("Ask must be >= bid".into())); }
        if self.accepted == 0 && self.confidence > 0 { return Err(MitchError::InvalidFieldValue("Cannot have confidence without accepted providers".into())); }
        Ok(())
    }
}

impl fmt::Display for Index {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Copy fields from packed struct to avoid unaligned references
        let ticker = self.ticker;
        let bid = self.bid;
        let ask = self.ask;
        let ci = self.ci;
        let tick_count = self.tick_count;
        let confidence = self.confidence;
        let accepted = self.accepted;
        let rejected = self.rejected;
        write!(
            f,
            "INDEX | Ticker: {:#018X} | Mid: {:.5} | Bid: {:.5} | Ask: {:.5} | CI: {} ubp ({:.6}) | Spread: {:.2} bps | Ticks: {} | Confidence: {} | Accepted: {} | Rejected: {}",
            ticker,
            self.mid(),
            bid,
            ask,
            ci,
            self.ci_price(),
            self.spread_bps(),
            tick_count,
            confidence,
            accepted,
            rejected,
        )
    }
}

// SAFETY: Index is `#[repr(C, packed)]` with only POD fields; no padding bytes.
unsafe impl MitchBody for Index {
    const SIZE: usize = message_sizes::INDEX;
}
