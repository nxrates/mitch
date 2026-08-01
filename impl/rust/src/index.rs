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
//! 36     | confidence | 1    | u8    | FLAG-SELECTED, three states:
//!        |            |      |       | bit6 FLAG_CONF_ACTIVE set → PACKED:
//!        |            |      |       |   bits0..6 = active (ticking) leg count
//!        |            |      |       |   0..64, bit7 = fresh-weight-share OK.
//!        |            |      |       | else bit3 FLAG_CONF_FRESHNESS set →
//!        |            |      |       |   legacy freshness fraction byte/255.
//!        |            |      |       | else → legacy active-provider COUNT
//!        |            |      |       |   (resampled / healed rows).
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
//!                                              (LEGACY: `confidence` is the
//!                                               freshness fraction byte/255;
//!                                               no live writer sets it — kept
//!                                               normative for historical rows)
//!                                       bit 6: FLAG_CONF_ACTIVE
//!                                              (`confidence` is the packed
//!                                               active-leg count + fresh-weight
//!                                               bit; set by aggregating
//!                                               writers. Numerically equals the
//!                                               *Bar*-space FLAG_S10_FLAT_FILL;
//!                                               safe because Index.flags is
//!                                               never copied into Bar.flags)
//!                                       bits 2,4,5,7: reserved for INDEX records
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
    /// Liveness metadata (1 byte). MEANING IS FLAG-SELECTED — three states, and
    /// a reader MUST branch on the flags in this order:
    ///
    /// 1. `FLAG_CONF_ACTIVE` (index flag bit 6) set — CURRENT live encoding:
    ///    bits 0..6 = `active_count`, the number of legs genuinely TICKING
    ///    (non-floored decay >= 0.1), saturated at [`CONF_MAX_ACTIVE_COUNT`];
    ///    bit 7 = `fresh_weight_ok`, the fresh-weight share cleared
    ///    `nxr_sdk::shard::FRESH_WEIGHT_SHARE_FLOOR`. (De)coders:
    ///    [`conf_pack_active`] / [`conf_active_count`] / [`conf_fresh_weight_ok`].
    /// 2. else `FLAG_CONF_FRESHNESS` (bit 3) set — LEGACY freshness fraction
    ///    `f = byte / 255 ∈ [0,1]` (historical rows written before the
    ///    2026-07-25 cutover, and replay of them). Decoder: [`conf_from_u8`].
    /// 3. else — legacy integer active-provider COUNT. What `resample_idx` /
    ///    heal leave on rewritten rows (decay inputs are not persisted, so a
    ///    marker is never fabricated). Carries NO liveness guarantee.
    ///
    /// ⚠ Neither encoding is a quality score. The state-2 FRACTION in particular
    /// is ANTI-correlated with feed breadth: every extra venue adds a leg that
    /// usually sits between ticks, and forwarder BBO-dedup makes an unchanged
    /// deep book indistinguishable from a dead one. Measured live 2026-07-25: a
    /// 1-leg Pyth composite (PAXG/USD) scores 237-254 while the deepest books
    /// score 15-61 (BTC/USDC, 10 venues) and 21-67 (ETH/USDC, 10 venues) — and
    /// the corrupt PAXG/USDT composite (one venue quoting a 1026 bps book)
    /// scores up to 255. That inversion is exactly why state 1 publishes a COUNT
    /// of ticking legs instead. Still gate breadth on [`Self::accepted`] +
    /// `active_count`, recency on the record/provider timestamp, and AGREEMENT
    /// on [`Self::ci`].
    pub confidence: u8,
    /// Accepted providers (1 byte)
    pub accepted: u8,
    /// Rejected providers (1 byte)
    pub rejected: u8,
    /// Flags bitfield (1 byte). See module-level docs and `model/index.md`
    /// for bit assignments (bit 0 heartbeat sentinel = `0b0000_0001`, bit 1
    /// historical backfill = `0b0000_0010`, bit 3 legacy conf-freshness, bit 4
    /// idx-healed, bit 5 no-book, bit 6 conf-active). Bits 2 and 7 are reserved
    /// for INDEX records and must be written as 0.
    pub flags: u8,
}

// Compile-time size assertion
const _: () = assert!(
    core::mem::size_of::<Index>() == 40,
    "Index must be exactly 40 bytes"
);

/// Wire scale for the `Index::confidence` freshness byte: a freshness
/// `f ∈ [0,1]` is stored as `round(f · 255)` (full u8 precision) and recovered
/// as `byte / 255`. Display as a 0-100 percent via `byte * 100 / 255`.
pub const MITCH_CONF_SCALE: f64 = 255.0;

/// Encode a freshness float `f ∈ [0,1]` to the wire byte (`round(f·255)`).
#[inline]
pub fn conf_to_u8(f: f64) -> u8 {
    (f.clamp(0.0, 1.0) * MITCH_CONF_SCALE).round() as u8
}

/// Decode a wire byte back to a freshness float `∈ [0,1]` (`byte / 255`).
#[inline]
pub fn conf_from_u8(b: u8) -> f64 {
    b as f64 / MITCH_CONF_SCALE
}

/// Mask of the `active_count` field in the packed `confidence` byte (bits 0..6).
pub const CONF_ACTIVE_COUNT_MASK: u8 = 0b0111_1111;

/// Bit 7 of the packed `confidence` byte: the composite's fresh-WEIGHT share
/// cleared its floor (`nxr_sdk::shard::FRESH_WEIGHT_SHARE_FLOOR`).
pub const CONF_FRESH_WEIGHT_BIT: u8 = 0b1000_0000;

/// Saturation ceiling for the packed `active_count`. Equals
/// `core::aggregator::MAX_ACCEPTED_PROVIDERS` (64), which is what leaves bit 7
/// free for [`CONF_FRESH_WEIGHT_BIT`]. Raising the aggregator constant above 127
/// collides — re-encode first.
pub const CONF_MAX_ACTIVE_COUNT: u32 = 64;

/// Pack the ACTIVE-provider measurement into the `confidence` wire byte
/// (`nxr_sdk::shard::FLAG_CONF_ACTIVE` semantics): `active_count` (saturated at
/// [`CONF_MAX_ACTIVE_COUNT`]) in bits 0..6, `fresh_weight_ok` in bit 7.
///
/// Companion decoders: [`conf_active_count`] / [`conf_fresh_weight_ok`]. The
/// historical fraction path keeps using [`conf_to_u8`] / [`conf_from_u8`].
#[inline]
pub fn conf_pack_active(active_count: u32, fresh_weight_ok: bool) -> u8 {
    let n = active_count.min(CONF_MAX_ACTIVE_COUNT) as u8;
    n | if fresh_weight_ok {
        CONF_FRESH_WEIGHT_BIT
    } else {
        0
    }
}

/// Number of genuinely-ticking legs from a packed `confidence` byte (bits 0..6).
/// Only meaningful when the record carries `nxr_sdk::shard::FLAG_CONF_ACTIVE`.
#[inline]
pub fn conf_active_count(b: u8) -> u32 {
    (b & CONF_ACTIVE_COUNT_MASK) as u32
}

/// Fresh-weight-share verdict from a packed `confidence` byte (bit 7).
/// Only meaningful when the record carries `nxr_sdk::shard::FLAG_CONF_ACTIVE`.
#[inline]
pub fn conf_fresh_weight_ok(b: u8) -> bool {
    (b & CONF_FRESH_WEIGHT_BIT) != 0
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

    /// Spread in basis points: (ask - bid) / mid * 10000.
    ///
    /// Returns 0.0 for a non-positive mid rather than NaN/inf. This is the LIVE
    /// path (`integrity_check`, `sdk/python`); the guard was previously only on
    /// the sibling `Tick::spread_bps`.
    pub fn spread_bps(&self) -> f64 {
        let mid = self.mid();
        if mid <= 0.0 {
            return 0.0;
        }
        (self.ask - self.bid) / mid * 10000.0
    }

    /// Volume imbalance: (vask - vbid) / (vask + vbid)
    pub fn volume_imbalance(&self) -> f64 {
        let total = self.vask as f64 + self.vbid as f64;
        if total == 0.0 {
            return 0.0;
        }
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
    /// NOTE: `confidence` is INDEPENDENT of `accepted` under both flagged
    /// encodings (`FLAG_CONF_ACTIVE` packed count+bit7, or legacy
    /// `FLAG_CONF_FRESHNESS` fraction), so the old `confidence <= accepted` and
    /// `accepted==0 && confidence>0` cross-constraints have been removed — a
    /// fully-stale single-provider record can legitimately have low freshness
    /// with `accepted > 0`, and a packed byte with bit 7 set exceeds any count.
    pub fn validate(&self) -> Result<(), MitchError> {
        match self.reject_reason() {
            Some(r) => Err(MitchError::InvalidFieldValue(r.into())),
            None => Ok(()),
        }
    }

    /// Single source of truth for the reject sites above: `None` = valid,
    /// `Some(label)` = a low-cardinality `&'static str` suitable both as a
    /// Prometheus label value and as the `validate()` error text (same
    /// `label()` convention as `nxr_sdk::udp_auth::AuthReject`). Callers on the
    /// UDP hot path use this instead of `validate()` so a rising reject count
    /// is attributable to a reason without parsing an error string.
    pub fn reject_reason(&self) -> Option<&'static str> {
        // Copy out of the packed struct so we can do float math without
        // triggering unaligned-reference lints.
        let bid = self.bid;
        let ask = self.ask;
        let ticker = self.ticker;

        if ticker == 0 {
            return Some("zero_ticker");
        }
        if !bid.is_finite() || !ask.is_finite() {
            return Some("non_finite");
        }
        if bid <= 0.0 || ask <= 0.0 {
            return Some("non_positive");
        }
        if ask < bid {
            return Some("crossed");
        }
        if bid > MAX_PRICE || ask > MAX_PRICE {
            return Some("above_max_price");
        }

        const MAX_SPREAD_BPS: f64 = 2000.0;
        let mid = (bid + ask) / 2.0;
        if mid > 0.0 && (ask - bid) / mid * 10_000.0 > MAX_SPREAD_BPS {
            return Some("spread_cap");
        }
        None
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
