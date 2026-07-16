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
//! 36     | confidence | 1    | u8    | Aggregate freshness percent, 0 to 100:
//!        |            |      |       | f = byte/100 (∈[0,1]) when FLAG_CONF_FRESHNESS
//!        |            |      |       | (index flag bit 3) is set; legacy active-
//!        |            |      |       | provider count when that flag is clear.
//! 37     | accepted   | 1    | u8    | Accepted providers
//! 38     | rejected   | 1    | u8    | Rejected providers
//! 39     | flags      | 1    | u8    | Bitfield:
//!                                       bit 0: FLAG_HEARTBEAT_SENTINEL
//!                                              (liveness-only, no real quote
//!                                               change; emitted by the live
//!                                               writer every 60s while quiet)
//!                                       bit 1: FLAG_HISTORICAL_BACKFILL
//!                                              (record produced by offline
//!                                               backfill / migrate / merge,
//!                                               not by the live aggregator)
//!                                       bit 3: FLAG_CONF_FRESHNESS
//!                                              (the `confidence` byte is a
//!                                               freshness percent 0-100, f=byte/100,
//!                                               not the legacy active-provider count;
//!                                               set by aggregating writers)
//!                                       bits 2,4-7: reserved for INDEX records
//!                                              (bit 2 is FLAG_RENKO_SYNTHETIC_-
//!                                               BRICK in the *Bar* flag space)
//! ```
//! Bit assignments are normative in `model/index.md`; writers and readers
//! must stay in lock-step with that spec.

use crate::body::MitchBody;
use crate::common::{message_sizes, MitchError};
use core::fmt;

/// Absolute price sanity ceiling shared by every carry-forward / gap-fill /
/// synth-multiply site downstream of `Index::validate` — no instrument we
/// quote prices in the billions, so this catches finite-but-astronomical
/// garbage that `is_finite()` alone admits (see `validate()` doc + incident
/// 2026-07-10).
pub const MAX_PRICE: f64 = 1.0e9;

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
    /// The reference encode / decode helpers are
    /// [`crate::ci_encode`] / [`crate::ci_decode`].
    /// `Self::ci_price()` applies the inverse for you.
    pub ci: u16,
    /// Raw ticks in aggregation window (2 bytes)
    pub tick_count: u16,
    /// Aggregate freshness percent, 0 to 100 (1 byte): `f = byte / 100 ∈ [0,1]`
    /// (or read the byte directly as a percent) when the record's
    /// `FLAG_CONF_FRESHNESS` (index flag bit 3) is set: ~100 when all providers
    /// are fresh, falling as components decay. When that flag is clear this is
    /// the legacy integer active-provider count.
    /// See [`conf_to_u8`] / [`conf_from_u8`] for the percent (de)coders.
    pub confidence: u8,
    /// Accepted providers (1 byte)
    pub accepted: u8,
    /// Rejected providers (1 byte)
    pub rejected: u8,
    /// Flags bitfield (1 byte). See module-level docs and `model/index.md`
    /// for bit assignments (bit 0 heartbeat sentinel = `0b0000_0001`, bit 1
    /// historical backfill = `0b0000_0010`, bit 3 conf-freshness). Bits 2,
    /// 4-7 are reserved and must be written as 0.
    pub flags: u8,
}

// Compile-time size assertion
const _: () = assert!(core::mem::size_of::<Index>() == 40, "Index must be exactly 40 bytes");

/// Percent scale for the `Index::confidence` freshness byte: a freshness
/// `f ∈ [0,1]` is stored as `round(f · 100)` (a percent 0-100) and recovered
/// as `byte / 100`.
pub const MITCH_CONF_SCALE: f64 = 100.0;

/// Encode a freshness float `f ∈ [0,1]` to the wire byte as a percent (`round(f·100)`).
#[inline]
pub fn conf_to_u8(f: f64) -> u8 {
    (f.clamp(0.0, 1.0) * MITCH_CONF_SCALE).round() as u8
}

/// Decode a percent wire byte back to a freshness float `∈ [0,1]` (`byte / 100`).
#[inline]
pub fn conf_from_u8(b: u8) -> f64 {
    b as f64 / MITCH_CONF_SCALE
}

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
            flags: 0,
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
        let ci_ubp = crate::common::ci_decode(self.ci);
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

    /// Validate message data integrity.
    ///
    /// Reject sites:
    /// - Zero ticker, non-positive or non-finite bid/ask, crossed quote.
    /// - `bid/ask > MAX_PRICE` (1e9 cap: no real instrument we quote prices in
    ///   the billions; catches finite-but-astronomical garbage — e.g. a torn
    ///   read or fail-open outlier-gate admit during a peer-thin restart — that
    ///   `is_finite()` alone lets through. Incident 2026-07-10: one such tick
    ///   admitted for USDC/USDT during an OOM crash-loop got carried forward as
    ///   `last_close` bar after bar (self-reinforcing, no upper-bound check
    ///   anywhere downstream) and cascaded into ZEC/USDC, XAUT/USDC synth
    ///   crosses. This is the earliest choke point — reject here first.
    /// - `spread_bps > MAX_SPREAD_BPS` (20% cap: thin enough to reject corrupted
    ///   feeds, wide enough to admit the widest illiquid pairs).
    ///
    /// NOTE: `confidence` is now an INDEPENDENT freshness percent byte (see
    /// `FLAG_CONF_FRESHNESS`), so the old `confidence <= accepted` and
    /// `accepted==0 && confidence>0` cross-constraints have been removed — a
    /// fully-stale single-provider record can legitimately have low freshness
    /// with `accepted > 0`, and freshness no longer counts providers.
    pub fn validate(&self) -> Result<(), MitchError> {
        // Copy out of the packed struct so we can do float math without
        // triggering unaligned-reference lints.
        let bid = self.bid;
        let ask = self.ask;
        let ticker = self.ticker;

        if ticker == 0 { return Err(MitchError::InvalidFieldValue("Ticker cannot be zero".into())); }
        if !bid.is_finite() { return Err(MitchError::InvalidFieldValue("Bid must be finite".into())); }
        if !ask.is_finite() { return Err(MitchError::InvalidFieldValue("Ask must be finite".into())); }
        if bid <= 0.0 { return Err(MitchError::InvalidFieldValue("Bid price must be positive".into())); }
        if ask <= 0.0 { return Err(MitchError::InvalidFieldValue("Ask price must be positive".into())); }
        if ask < bid { return Err(MitchError::InvalidFieldValue("Ask must be >= bid".into())); }
        if bid > MAX_PRICE || ask > MAX_PRICE {
            return Err(MitchError::InvalidFieldValue("price exceeds 1e9 sanity cap".into()));
        }

        const MAX_SPREAD_BPS: f64 = 2000.0;
        let mid = (bid + ask) / 2.0;
        if mid > 0.0 {
            let spread_bps = (ask - bid) / mid * 10_000.0;
            if spread_bps > MAX_SPREAD_BPS {
                return Err(MitchError::InvalidFieldValue("spread exceeds 2000 bps cap".into()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_price_above_max() {
        // 2026-07-10 incident regression: a finite-but-astronomical price must
        // be rejected at the earliest gate, not just non-finite ones.
        let bad = Index::new(1, MAX_PRICE * 2.0, MAX_PRICE * 2.0, 0, 0, 0, 0, 100, 1, 0);
        assert!(bad.validate().is_err());
    }

    #[test]
    fn validate_accepts_price_at_max() {
        let ok = Index::new(1, MAX_PRICE, MAX_PRICE, 0, 0, 0, 0, 100, 1, 0);
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn validate_accepts_normal_price() {
        let ok = Index::new(1, 1.0006, 1.0008, 0, 0, 0, 0, 100, 1, 0);
        assert!(ok.validate().is_ok());
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
