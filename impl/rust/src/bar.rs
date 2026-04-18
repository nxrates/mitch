//! Bar - canonical enriched bar format (128 bytes, 2 cache lines).
//!
//! Supports both time-based (kline) and price-based (renko) bars.
//! - **Kline**: `open_ts`/`close_ts` define the time bucket (u48 ticks since 2010).
//! - **Renko**: `open`/`close` define the brick; for bullish bars `high == close`
//!   (wick is `low`), for bearish bars `low == close` (wick is `high`).
//!
//! Timestamps are stored as 6-byte u48 little-endian tick values using the
//! [`crate::timestamp`] encoding (16 µs ticks since 2010-01-01).
//!
//! Shared between series-factory (writer) and btr/prime (reader).
//! Pod + Zeroable via bytemuck for safe zero-copy I/O.

use crate::body::MitchBody;
use crate::common::{message_sizes, MitchError};
use crate::timestamp;

/// Size of a serialized Bar in bytes.
pub const BAR_SIZE: usize = message_sizes::BAR;

/// Enriched bar record (128 bytes = 2 cache lines).
///
/// ## Cache line 1 - Core OHLCV + timing (64B)
/// ```text
/// Offset | Field      | Size | Type     | Description
/// -------|------------|------|----------|-------------------------------
/// 0      | open_ts    | 6    | [u8; 6]  | u48 LE ticks since 2010
/// 6      | close_ts   | 6    | [u8; 6]  | u48 LE ticks since 2010
/// 12     | open       | 8    | f64      | Open price
/// 20     | high       | 8    | f64      | High price
/// 28     | low        | 8    | f64      | Low price
/// 36     | close      | 8    | f64      | Close price
/// 44     | vbid       | 4    | u32      | Cumulative bid volume
/// 48     | vask       | 4    | u32      | Cumulative ask volume
/// 52     | tick_count | 4    | u32      | Ticks in bar
/// 56     | _pad       | 8    | [u8; 8]  | Padding to 64B
/// ```
///
/// ## Cache line 2 - Microstructure features (64B)
/// ```text
/// Offset | Field          | Size | Type  | Description
/// -------|----------------|------|-------|-------------------------------
/// 64     | dispersion     | 4    | f32   | σ/μ Welford coefficient of variation
/// 68     | drift          | 4    | f32   | Normalized OLS slope (slope/mean)
/// 72     | vwap_dev       | 4    | f32   | (close − quoteVWAP) / close
/// 76     | price_impact   | 4    | f32   | ΔPrice / vol_imbalance × 1e6
/// 80     | vol_imbalance  | 4    | f32   | (vask − vbid) / (vask + vbid)
/// 84     | tick_efficiency | 4   | f32   | |ΔPrice| / (price × tick_count)
/// 88     | log_volume     | 4    | f32   | ln(total_vol + 1)
/// 92     | _reserved      | 36   | [u8; 36] | Reserved (zero)
/// ```
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Bar {
    // ── Cache line 1: core OHLCV + timing (64B) ────────────────────
    /// Bar open timestamp (u48 LE ticks since 2010).
    pub open_ts: [u8; 6],
    /// Bar close timestamp (u48 LE ticks since 2010).
    pub close_ts: [u8; 6],
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    /// Cumulative bid volume in bar.
    pub vbid: u32,
    /// Cumulative ask volume in bar.
    pub vask: u32,
    /// Number of ticks in bar.
    pub tick_count: u32,
    /// Padding to align cache line 1 to 64 bytes.
    pub _pad: [u8; 8],

    // ── Cache line 2: microstructure features (64B) ─────────────────
    /// σ/μ - Welford coefficient of variation.
    pub dispersion: f32,
    /// Normalized OLS slope (slope / mean, no multiplication by n).
    pub drift: f32,
    /// (close − quoteVWAP) / close.
    pub vwap_dev: f32,
    /// ΔPrice / vol_imbalance × 1e6 (formerly kyle_lambda).
    pub price_impact: f32,
    /// (vask − vbid) / (vask + vbid) (formerly ofi).
    pub vol_imbalance: f32,
    /// |ΔPrice| / (price × tick_count).
    pub tick_efficiency: f32,
    /// ln(total_vol + 1).
    pub log_volume: f32,
    /// Reserved for future use (zero-filled).
    pub _reserved: [u8; 36],
}

// Compile-time size assertion.
const _: () = assert!(core::mem::size_of::<Bar>() == 128);

impl Bar {
    /// Create a minimal bar (zeroed enrichment fields).
    ///
    /// Timestamps are mts values (see [`crate::timestamp::from_epoch_ms`]).
    #[inline]
    pub const fn new_ohlcv(
        open_mts: u64,
        close_mts: u64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        vbid: u32,
        vask: u32,
        tick_count: u32,
    ) -> Self {
        Self {
            open_ts: timestamp::encode_u48(open_mts),
            close_ts: timestamp::encode_u48(close_mts),
            open,
            high,
            low,
            close,
            vbid,
            vask,
            tick_count,
            _pad: [0; 8],
            dispersion: 0.0,
            drift: 0.0,
            vwap_dev: 0.0,
            price_impact: 0.0,
            vol_imbalance: 0.0,
            tick_efficiency: 0.0,
            log_volume: 0.0,
            _reserved: [0; 36],
        }
    }

    // ── Timestamp accessors ─────────────────────────────────────────

    /// Read open timestamp as decoded mts value.
    #[inline]
    pub fn open_mts(&self) -> u64 {
        timestamp::decode_u48(&self.open_ts)
    }

    /// Read close timestamp as decoded mts value.
    #[inline]
    pub fn close_mts(&self) -> u64 {
        timestamp::decode_u48(&self.close_ts)
    }

    /// Open time as Unix-epoch milliseconds.
    #[inline]
    pub fn open_time_ms(&self) -> i64 {
        timestamp::to_epoch_ms(self.open_mts())
    }

    /// Close time as Unix-epoch milliseconds.
    #[inline]
    pub fn close_time_ms(&self) -> i64 {
        timestamp::to_epoch_ms(self.close_mts())
    }

    /// Set open timestamp from an mts value.
    #[inline]
    pub fn set_open_mts(&mut self, mts: u64) {
        self.open_ts = timestamp::encode_u48(mts);
    }

    /// Set close timestamp from an mts value.
    #[inline]
    pub fn set_close_mts(&mut self, mts: u64) {
        self.close_ts = timestamp::encode_u48(mts);
    }

    // ── Serialization ───────────────────────────────────────────────

    /// Pack to bytes (zero-copy).
    pub fn pack(&self) -> [u8; message_sizes::BAR] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Unpack from bytes.
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::BAR {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::BAR,
                actual: bytes.len(),
            });
        }
        Ok(unsafe { (bytes.as_ptr() as *const Self).read_unaligned() })
    }

    /// Unpack without bounds check.
    #[inline]
    pub unsafe fn unpack_unchecked(bytes: &[u8]) -> Self {
        (bytes.as_ptr() as *const Self).read_unaligned()
    }

    // ── Derived metrics ─────────────────────────────────────────────

    /// Bar duration in milliseconds.
    #[inline]
    pub fn duration_ms(&self) -> i64 {
        self.close_time_ms() - self.open_time_ms()
    }

    /// True if close > open.
    #[inline]
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    /// Mid price at close: (open + close) / 2.
    #[inline]
    pub fn mid_price(&self) -> f64 {
        (self.open + self.close) / 2.0
    }

    /// Total volume (bid + ask).
    #[inline]
    pub fn total_volume(&self) -> u64 {
        self.vbid as u64 + self.vask as u64
    }

    /// Volume imbalance derived from fields (−1.0 … 1.0).
    #[inline]
    pub fn volume_imbalance_derived(&self) -> f64 {
        let total = self.total_volume() as f64;
        if total > 0.0 {
            (self.vask as f64 - self.vbid as f64) / total
        } else {
            0.0
        }
    }

    /// Log return: ln(close / open).
    #[inline]
    pub fn log_return(&self) -> f64 {
        if self.open > 0.0 {
            (self.close / self.open).ln()
        } else {
            0.0
        }
    }

    /// Range ratio: (high − low) / close. Replaces the old `spread` field.
    #[inline]
    pub fn range_ratio(&self) -> f64 {
        if self.close > 0.0 {
            (self.high - self.low) / self.close
        } else {
            0.0
        }
    }

    /// Struct size.
    pub const fn size() -> usize {
        message_sizes::BAR
    }
}

// SAFETY: Bar is `#[repr(C, packed)]` with only POD fields; no padding bytes.
unsafe impl MitchBody for Bar {
    const SIZE: usize = message_sizes::BAR;
}

// ═════════════════════════════════════════════════════════════════════
// TESTS
// ═════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{pack_batch, unpack_all};
    use crate::timestamp::from_epoch_ms;

    /// Helper: create a test bar with known ticks.
    fn test_bar() -> Bar {
        let open_mts = from_epoch_ms(1_744_372_800_000); // 2026-04-11 12:00:00 UTC
        let close_mts = from_epoch_ms(1_744_372_860_000); // +60s
        Bar::new_ohlcv(
            open_mts,
            close_mts,
            100.0,
            105.0,
            99.0,
            103.0,
            1000,
            1200,
            50,
        )
    }

    #[test]
    fn size_is_128() {
        assert_eq!(core::mem::size_of::<Bar>(), 128);
    }

    #[test]
    fn round_trip() {
        let bar = test_bar();
        let packed = bar.pack();
        assert_eq!(packed.len(), 128);
        let unpacked = Bar::unpack(&packed).unwrap();
        assert_eq!(bar, unpacked);
    }

    #[test]
    fn timestamp_encode_decode() {
        let bar = test_bar();
        let open_mts = bar.open_mts();
        let close_mts = bar.close_mts();

        // Ticks should round-trip through the u48 encoding.
        assert_eq!(open_mts, from_epoch_ms(1_744_372_800_000));
        assert_eq!(close_mts, from_epoch_ms(1_744_372_860_000));

        // Millisecond conversion should be within 1 ms tolerance.
        assert!((bar.open_time_ms() - 1_744_372_800_000).abs() <= 1);
        assert!((bar.close_time_ms() - 1_744_372_860_000).abs() <= 1);
    }

    #[test]
    fn set_timestamps() {
        let mut bar = test_bar();
        let new_ticks = from_epoch_ms(1_744_400_000_000);
        bar.set_open_mts(new_ticks);
        assert_eq!(bar.open_mts(), new_ticks);

        bar.set_close_mts(new_ticks + 1000);
        assert_eq!(bar.close_mts(), new_ticks + 1000);
    }

    #[test]
    fn duration() {
        let bar = test_bar();
        let expected = bar.close_time_ms() - bar.open_time_ms();
        assert_eq!(bar.duration_ms(), expected);
        // Should be roughly 60_000 ms (60 seconds).
        assert!((bar.duration_ms() - 60_000).abs() <= 1);
    }

    #[test]
    fn bullish_bearish() {
        let open_t = from_epoch_ms(1_700_000_000_000);
        let close_t = from_epoch_ms(1_700_000_001_000);
        let bull = Bar::new_ohlcv(open_t, close_t, 100.0, 105.0, 99.0, 103.0, 0, 0, 0);
        let bear = Bar::new_ohlcv(open_t, close_t, 103.0, 105.0, 99.0, 100.0, 0, 0, 0);
        assert!(bull.is_bullish());
        assert!(!bear.is_bullish());
    }

    #[test]
    fn mid_price() {
        let bar = test_bar();
        assert!((bar.mid_price() - 101.5).abs() < 1e-10);
    }

    #[test]
    fn total_volume_and_imbalance() {
        let bar = test_bar();
        assert_eq!(bar.total_volume(), 2200);
        let imb = bar.volume_imbalance_derived();
        // (1200 - 1000) / 2200 = 200/2200 ~ 0.0909
        assert!((imb - 200.0 / 2200.0).abs() < 1e-10);
    }

    #[test]
    fn log_return() {
        let bar = test_bar();
        let expected = (103.0_f64 / 100.0).ln();
        assert!((bar.log_return() - expected).abs() < 1e-10);
    }

    #[test]
    fn range_ratio() {
        let bar = test_bar();
        // (105 - 99) / 103
        let expected = 6.0 / 103.0;
        assert!((bar.range_ratio() - expected).abs() < 1e-10);
    }

    #[test]
    fn batch_round_trip() {
        let open_base = from_epoch_ms(1_700_000_000_000);
        let bars: Vec<Bar> = (0..20)
            .map(|i| {
                Bar::new_ohlcv(
                    open_base + i * 60_000,
                    open_base + (i + 1) * 60_000,
                    100.0 + i as f64,
                    105.0 + i as f64,
                    99.0 + i as f64,
                    103.0 + i as f64,
                    100 * i as u32,
                    120 * i as u32,
                    50,
                )
            })
            .collect();
        let packed = pack_batch(&bars);
        assert_eq!(packed.len(), 20 * 128);
        let unpacked: Vec<Bar> = unpack_all(&packed).unwrap();
        assert_eq!(bars, unpacked);
    }

    #[test]
    fn renko_bullish_layout() {
        let t = from_epoch_ms(1_700_000_000_000);
        let bar = Bar::new_ohlcv(t, t + 1, 100.0, 102.0, 98.0, 102.0, 0, 0, 10);
        assert!(bar.is_bullish());
        let (h, c) = (bar.high, bar.close);
        assert_eq!(h, c);
    }

    #[test]
    fn renko_bearish_layout() {
        let t = from_epoch_ms(1_700_000_000_000);
        let bar = Bar::new_ohlcv(t, t + 1, 102.0, 104.0, 100.0, 100.0, 0, 0, 10);
        assert!(!bar.is_bullish());
        let (l, c) = (bar.low, bar.close);
        assert_eq!(l, c);
    }

    #[test]
    fn unpack_too_small() {
        let buf = [0u8; 64]; // too small
        assert!(Bar::unpack(&buf).is_err());
    }

    #[test]
    fn zero_bar() {
        let bar = Bar::new_ohlcv(0, 0, 0.0, 0.0, 0.0, 0.0, 0, 0, 0);
        assert_eq!(bar.total_volume(), 0);
        assert_eq!(bar.volume_imbalance_derived(), 0.0);
        assert_eq!(bar.log_return(), 0.0);
        assert_eq!(bar.range_ratio(), 0.0);
    }

    #[test]
    fn u48_max_fits() {
        // Ensure u48 max (281 trillion) encodes and decodes correctly.
        let max_ticks: u64 = (1u64 << 48) - 1;
        let bar = Bar::new_ohlcv(max_ticks, max_ticks, 1.0, 1.0, 1.0, 1.0, 0, 0, 0);
        assert_eq!(bar.open_mts(), max_ticks);
        assert_eq!(bar.close_mts(), max_ticks);
    }
}
