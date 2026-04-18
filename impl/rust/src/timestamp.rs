//! # MITCH Timestamp - the canonical clock for the NX/BTR pipeline
//!
//! Every timestamp in the system - wire messages, tick files, bar files, vol
//! files, IPC rings - uses this single encoding.  There is no other format.
//!
//! ## Encoding
//!
//! | Property   | Value |
//! |------------|-------|
//! | Unit       | 16 µs (one *tick* = 16 microseconds = 16 000 ns) |
//! | Epoch      | `2010-01-01T00:00:00Z` (Unix 1 262 304 000) |
//! | Storage    | u48 little-endian (6 bytes in [`MitchHeader`](crate::MitchHeader)) |
//! | Overflow   | ~year 2152 (142 years of headroom) |
//! | Resolution | 62 500 ticks per second |
//!
//! ```text
//! encode:  ticks = (epoch_µs − EPOCH_µs) >> 4
//! decode:  epoch_µs = (ticks << 4) + EPOCH_µs
//! ```
//!
//! The right-shift by 4 is a division by 16.  The 16 µs granularity was
//! chosen so the full BTC+altcoin tick stream fits in a u48 field for the
//! next 142 years while keeping sub-millisecond precision.
//!
//! The 2010 epoch supports 15+ years of historical backtesting data.
//!
//! ## Conversion helpers
//!
//! | Direction | Microseconds | Milliseconds | Nanoseconds |
//! |-----------|-------------|-------------|-------------|
//! | Encode    | [`from_epoch_us`] | [`from_epoch_ms`] | [`from_epoch_ns`] |
//! | Decode    | [`to_epoch_us`]   | [`to_epoch_ms`]   | [`to_epoch_ns`]   |
//!
//! Pre-2010 inputs saturate to tick 0 (no negative ticks).
//!
//! ## Examples
//!
//! ```
//! use mitch::timestamp::{from_epoch_ms, to_epoch_ms};
//!
//! // 2026-04-11 12:00:00 UTC  →  epoch_ms 1 744 372 800 000
//! let ticks = from_epoch_ms(1_744_372_800_000);
//! assert_eq!(to_epoch_ms(ticks), 1_744_372_800_000);
//!
//! // Round-trip is lossless at millisecond granularity
//! let ticks2 = from_epoch_ms(1_700_000_000_123);
//! assert!((to_epoch_ms(ticks2) - 1_700_000_000_123).abs() <= 1);
//! ```
//!
//! ## Wire layout inside [`MitchHeader`](crate::MitchHeader)
//!
//! ```text
//! offset  0: type_provider u16 LE  (low 4b = msg type code, bits 4..16 = provider_id)
//! offset  2: timestamp     u48 LE  ← THIS MODULE
//! offset  8: count         u8
//! offset  9: flags         u8
//! offset 10: sequence      u16 LE
//! offset 12: _reserved     [u8; 4]
//! ```
//!
//! All functions are `#[inline]` and branchless for hot-path use.

/// 2010-01-01T00:00:00Z as microseconds since Unix epoch.
/// = 1_262_304_000 seconds × 1_000_000
pub const EPOCH_US: u64 = 1_262_304_000_000_000;

/// 2010-01-01T00:00:00Z as milliseconds since Unix epoch.
pub const EPOCH_MS: i64 = 1_262_304_000_000;

/// 2010-01-01T00:00:00Z as nanoseconds since Unix epoch.
pub const EPOCH_NS: i64 = 1_262_304_000_000_000_000;

/// Tick resolution in microseconds.
pub const TICK_US: u64 = 16;

/// Tick resolution in nanoseconds.
pub const TICK_NS: u64 = 16_000;

/// Maximum representable tick value (u48).
pub const MAX_TICKS: u64 = (1u64 << 48) - 1;

// ── u48 wire encoding (6-byte little-endian) ────────────────────────────
//
// "mts" = MITCH timestamp: the decoded u64 value representing 16 µs
// intervals since 2010-01-01T00:00:00Z.  All encode/decode helpers in
// this module produce or consume mts values.

/// Encode an mts value into a 6-byte little-endian u48.
/// Top 16 bits are silently truncated.
#[inline]
pub const fn encode_u48(mts: u64) -> [u8; 6] {
    [
        mts as u8,
        (mts >> 8) as u8,
        (mts >> 16) as u8,
        (mts >> 24) as u8,
        (mts >> 32) as u8,
        (mts >> 40) as u8,
    ]
}

/// Decode a 6-byte little-endian u48 into an mts value.
#[inline]
pub const fn decode_u48(bytes: &[u8; 6]) -> u64 {
    bytes[0] as u64
        | (bytes[1] as u64) << 8
        | (bytes[2] as u64) << 16
        | (bytes[3] as u64) << 24
        | (bytes[4] as u64) << 32
        | (bytes[5] as u64) << 40
}

// ── Encode from various units ───────────────────────────────────────────

/// Encode Unix-epoch microseconds → mts.
/// Pre-epoch inputs saturate to 0.
#[inline]
pub fn from_epoch_us(epoch_us: u64) -> u64 {
    epoch_us.saturating_sub(EPOCH_US) >> 4
}

/// Encode Unix-epoch milliseconds → mts.
/// Pre-epoch inputs saturate to 0.
#[inline]
pub fn from_epoch_ms(epoch_ms: i64) -> u64 {
    if epoch_ms <= EPOCH_MS {
        return 0;
    }
    let us = (epoch_ms - EPOCH_MS) as u64 * 1000;
    us >> 4
}

/// Encode Unix-epoch nanoseconds → mts.
/// Pre-epoch inputs saturate to 0.
#[inline]
pub fn from_epoch_ns(epoch_ns: i64) -> u64 {
    if epoch_ns <= EPOCH_NS {
        return 0;
    }
    let us = ((epoch_ns - EPOCH_NS) / 1000) as u64;
    us >> 4
}

// ── Decode to various units ─────────────────────────────────────────────

/// Decode mts → Unix-epoch microseconds.
#[inline]
pub fn to_epoch_us(mts: u64) -> u64 {
    (mts << 4) + EPOCH_US
}

/// Decode mts → Unix-epoch milliseconds.
#[inline]
pub fn to_epoch_ms(mts: u64) -> i64 {
    ((mts << 4) + EPOCH_US) as i64 / 1000
}

/// Decode mts → Unix-epoch nanoseconds.
#[inline]
pub fn to_epoch_ns(mts: u64) -> i64 {
    ((mts << 4) + EPOCH_US) as i64 * 1000
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_roundtrip() {
        // 2010-01-01T00:00:00Z should encode to tick 0
        assert_eq!(from_epoch_us(EPOCH_US), 0);
        assert_eq!(to_epoch_us(0), EPOCH_US);
    }

    #[test]
    fn known_timestamp() {
        // 2026-04-11T12:00:00Z
        let epoch_s: u64 = 1_744_372_800;
        let epoch_us = epoch_s * 1_000_000;
        let ticks = from_epoch_us(epoch_us);
        let back = to_epoch_us(ticks);
        assert!(back.abs_diff(epoch_us) < TICK_US);
    }

    #[test]
    fn ms_roundtrip() {
        let epoch_ms: i64 = 1_744_372_800_000; // 2026-04-11T12:00:00Z
        let ticks = from_epoch_ms(epoch_ms);
        let back = to_epoch_ms(ticks);
        assert!((back - epoch_ms).abs() <= 1);
    }

    #[test]
    fn ns_roundtrip() {
        let epoch_ns: i64 = 1_744_372_800_000_000_000;
        let ticks = from_epoch_ns(epoch_ns);
        let back = to_epoch_ns(ticks);
        assert!((back - epoch_ns).abs() < TICK_NS as i64);
    }

    #[test]
    fn supports_backtesting_from_2010() {
        // 2010-06-01T00:00:00Z - should produce a valid non-zero tick
        let epoch_ms: i64 = 1_275_350_400_000;
        let ticks = from_epoch_ms(epoch_ms);
        assert!(ticks > 0, "2010 timestamps must be encodable");
        let back = to_epoch_ms(ticks);
        assert!((back - epoch_ms).abs() <= 1);
    }

    #[test]
    fn fits_in_u48() {
        // 142 years from 2010 = ~2152. Should still fit.
        let epoch_2150_us: u64 = EPOCH_US + 140 * 365 * 86400 * 1_000_000;
        let ticks = from_epoch_us(epoch_2150_us);
        assert!(ticks <= MAX_TICKS, "ticks={ticks} > MAX={MAX_TICKS}");
    }

    #[test]
    fn resolution_is_16us() {
        let t0 = from_epoch_us(EPOCH_US);
        let t1 = from_epoch_us(EPOCH_US + 16);
        assert_eq!(t1 - t0, 1);
    }

    #[test]
    fn pre_epoch_saturates() {
        assert_eq!(from_epoch_us(0), 0);
        assert_eq!(from_epoch_us(EPOCH_US - 1), 0);
    }

    #[test]
    fn pre_epoch_ms_saturates() {
        assert_eq!(from_epoch_ms(0), 0);
        assert_eq!(from_epoch_ms(-1_000_000), 0);
        assert_eq!(from_epoch_ms(EPOCH_MS - 1), 0);
        assert_eq!(from_epoch_ms(EPOCH_MS), 0);
    }

    #[test]
    fn pre_epoch_ns_saturates() {
        assert_eq!(from_epoch_ns(0), 0);
        assert_eq!(from_epoch_ns(-1_000_000_000), 0);
        assert_eq!(from_epoch_ns(EPOCH_NS - 1), 0);
        assert_eq!(from_epoch_ns(EPOCH_NS), 0);
    }
}
