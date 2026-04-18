//! MITCH frame types - `[MitchHeader 16B][Body]` composition.
//!
//! A frame is the canonical on-wire and on-disk representation of a MITCH
//! message with a single body entry (`count = 1`). The timestamp lives in the
//! header, never embedded in the body.
//!
//! All frame types are `#[repr(C, packed)]`, `Pod`, and `Zeroable` for
//! zero-copy file I/O via `bytemuck::cast_slice`.
//!
//! # Timestamp encoding
//!
//! The header carries a u48 value in **16µs ticks since 2010-01-01T00:00:00Z**.
//! Resolution: 16 microseconds. Overflow: ~2152.
//!
//! Convert with `mitch::timestamp::{from_epoch_ms, from_epoch_ns, to_epoch_ms, ...}`.

use crate::common::{message_type, message_sizes, MitchError};
use crate::header::MitchHeader;
use crate::tick::Tick;

// =============================================================================
// TICK FRAME (48 bytes)
// =============================================================================

/// Tick frame: `[MitchHeader 16B][Tick 32B]` = 48 bytes.
///
/// The timestamp lives in the header, not embedded in the tick body.
///
/// ```text
/// Offset | Field  | Size | Description
/// -------|--------|------|---------------------------
/// 0      | header | 16   | MitchHeader (type='s')
/// 16     | body   | 32   | Tick
/// ```
///
/// # File format
///
/// Flat array of 48-byte records. Count = `file_size / 48`.
/// Supports zero-copy mmap via `bytemuck::cast_slice::<u8, TickFrame>`.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct TickFrame {
    /// 16-byte MITCH header (type = 's', 16µs ticks since 2010, count = 1)
    pub header: MitchHeader,
    /// 32-byte tick body
    pub body: Tick,
}

/// Size of a TickFrame in bytes.
pub const TICK_FRAME_SIZE: usize = message_sizes::HEADER + message_sizes::TICK;

impl TickFrame {
    /// Create a new TickFrame.
    ///
    /// # Arguments
    /// * `provider_id` - data source provider ID (0-4095)
    /// * `ticks` - 16us ticks since 2010-01-01 (u48). Use `mitch::timestamp::from_epoch_*`.
    /// * `body` - tick data
    #[inline]
    pub fn new(provider_id: u16, ticks: u64, body: Tick) -> Self {
        Self {
            header: MitchHeader::new(message_type::TICK, provider_id, ticks, 1),
            body,
        }
    }

    /// Raw u48 timestamp (16µs ticks since 2010). Decode with `mitch::timestamp::to_epoch_*`.
    #[inline]
    pub fn timestamp(&self) -> u64 {
        self.header.get_timestamp()
    }

    /// Timestamp as Unix epoch milliseconds (convenience).
    #[inline]
    pub fn timestamp_ms(&self) -> i64 {
        crate::timestamp::to_epoch_ms(self.header.get_timestamp())
    }

    /// Provider ID from header.
    #[inline]
    pub fn provider_id(&self) -> u16 {
        self.header.provider_id()
    }

    /// Get the tick body (copy - packed struct cannot yield references).
    #[inline]
    pub fn tick(&self) -> Tick {
        self.body
    }

    /// Mid price convenience.
    #[inline]
    pub fn mid_price(&self) -> f64 {
        self.body.mid_price()
    }

    /// Spread convenience (ask − bid).
    #[inline]
    pub fn spread(&self) -> f64 {
        self.body.spread()
    }

    /// Total volume (bid + ask).
    #[inline]
    pub fn total_volume(&self) -> u64 {
        self.body.total_volume()
    }

    /// Volume imbalance (−1.0 … 1.0).
    #[inline]
    pub fn volume_imbalance(&self) -> f64 {
        self.body.volume_imbalance()
    }

    /// Pack to bytes (zero-copy transmute).
    pub fn pack(&self) -> [u8; TICK_FRAME_SIZE] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Unpack from bytes with validation.
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < TICK_FRAME_SIZE {
            return Err(MitchError::BufferTooSmall {
                expected: TICK_FRAME_SIZE,
                actual: bytes.len(),
            });
        }
        let frame: Self = unsafe { (bytes.as_ptr() as *const Self).read_unaligned() };
        let mt = frame.header.message_type();
        if mt != message_type::TICK {
            return Err(MitchError::InvalidMessageType(mt));
        }
        Ok(frame)
    }

    /// Unpack without validation (maximum performance).
    ///
    /// # Safety
    /// Caller must ensure `bytes.len() >= 48` and data is a valid TickFrame.
    #[inline]
    pub unsafe fn unpack_unchecked(bytes: &[u8]) -> Self {
        (bytes.as_ptr() as *const Self).read_unaligned()
    }

    /// Struct size.
    pub const fn size() -> usize {
        TICK_FRAME_SIZE
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_frame_size() {
        assert_eq!(core::mem::size_of::<TickFrame>(), 48);
        assert_eq!(TICK_FRAME_SIZE, 48);
    }

    #[test]
    fn tick_frame_round_trip() {
        use crate::timestamp;
        let ticks = timestamp::from_epoch_ms(1_744_364_200_000);
        let tick = Tick::new_unchecked(1, 100.0, 100.05, 500, 600);
        let frame = TickFrame::new(101, ticks, tick); // provider_id = 101 (Binance)
        let packed = frame.pack();
        let unpacked = TickFrame::unpack(&packed).unwrap();
        assert_eq!(frame, unpacked);
        assert_eq!(unpacked.provider_id(), 101);
    }

    #[test]
    fn tick_frame_timestamp() {
        use crate::timestamp;
        let epoch_ms: i64 = 1_744_364_200_000;
        let ticks = timestamp::from_epoch_ms(epoch_ms);
        let tick = Tick::new_unchecked(1, 100.0, 100.05, 0, 0);
        let frame = TickFrame::new(0, ticks, tick);
        assert_eq!(frame.timestamp(), ticks);
        let back_ms = timestamp::to_epoch_ms(frame.timestamp());
        assert!((back_ms - epoch_ms).abs() <= 1);
    }

    #[test]
    fn tick_frame_accessors() {
        let tick = Tick::new_unchecked(42, 100.0, 100.10, 500, 600);
        let frame = TickFrame::new(0, 0, tick);
        assert!((frame.mid_price() - 100.05).abs() < 1e-10);
        assert!((frame.spread() - 0.10).abs() < 1e-10);
        assert_eq!(frame.total_volume(), 1100);
    }

    #[test]
    fn tick_frame_provider_id() {
        let tick = Tick::new_unchecked(1, 100.0, 100.05, 0, 0);
        let frame = TickFrame::new(1301, 0, tick); // max current provider (XT)
        assert_eq!(frame.provider_id(), 1301);
    }

    #[test]
    fn tick_frame_invalid_type() {
        let tick = Tick::new_unchecked(1, 100.0, 100.05, 0, 0);
        let frame = TickFrame::new(0, 0, tick);
        let mut packed = frame.pack();
        // Corrupt the message type code (low nibble of byte 0)
        packed[0] = (packed[0] & 0xF0) | 0x0F; // invalid code 15
        assert!(TickFrame::unpack(&packed).is_err());
    }

    #[test]
    fn tick_frame_buffer_too_small() {
        let bytes = [0u8; 47];
        assert!(TickFrame::unpack(&bytes).is_err());
    }

    #[cfg(feature = "bytemuck")]
    #[test]
    fn tick_frame_bytemuck_cast() {
        let tick = Tick::new_unchecked(1, 100.0, 100.05, 500, 600);
        let frames = vec![
            TickFrame::new(101, 100, tick),
            TickFrame::new(101, 200, tick),
            TickFrame::new(101, 300, tick),
        ];
        let bytes: &[u8] = bytemuck::cast_slice(&frames);
        assert_eq!(bytes.len(), 144); // 3 × 48
        let back: &[TickFrame] = bytemuck::cast_slice(bytes);
        assert_eq!(back.len(), 3);
        assert_eq!(back[0], frames[0]);
        assert_eq!(back[2], frames[2]);
    }
}
