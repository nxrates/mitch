//! MITCH frame types: `[MitchHeader 16B][Body N B]` composition.
//!
//! A frame is the canonical on-wire and on-disk representation of a MITCH
//! message carrying a single body entry (`count = 1`). The timestamp lives in
//! the header, never embedded in the body.
//!
//! A single generic `Frame<B>` wrapper covers every body type; concrete on-wire
//! variants are exposed as type aliases (`TickFrame`, `TradeFrame`, `BarFrame`).
//! All frames are `#[repr(C, packed)]`, `Pod`, and `Zeroable` for zero-copy
//! file I/O via `bytemuck::cast_slice`.
//!
//! # Timestamp encoding
//!
//! The header carries a u48 value in **16µs ticks since 2010-01-01T00:00:00Z**.
//! Resolution: 16 microseconds. Overflow: ~2152.
//!
//! Convert with `mitch::timestamp::{from_epoch_ms, from_epoch_ns, to_epoch_ms, ...}`.

use crate::bar::Bar;
use crate::body::MitchBody;
use crate::common::{message_sizes, message_type, MitchError};
use crate::header::MitchHeader;
use crate::heartbeat::Heartbeat;
use crate::tick::Tick;
use crate::trade::Trade;

// =============================================================================
// FRAME BODY TRAIT
// =============================================================================

/// A MITCH body type that can be carried in a [`Frame<B>`].
///
/// # Safety
///
/// Implementers must be `#[repr(C, packed)]`, `Copy`, and `MSG_TYPE` must match
/// the on-wire discriminator the body encodes. The generic frame wrapper relies
/// on both invariants when `transmute`-casting to/from bytes.
pub unsafe trait FrameBody: MitchBody {
    /// MITCH message-type byte stamped into the header.
    const MSG_TYPE: u8;
}

unsafe impl FrameBody for Tick {
    const MSG_TYPE: u8 = message_type::TICK;
}
unsafe impl FrameBody for Trade {
    const MSG_TYPE: u8 = message_type::TRADE;
}
unsafe impl FrameBody for Bar {
    const MSG_TYPE: u8 = message_type::BAR;
}
unsafe impl FrameBody for Heartbeat {
    const MSG_TYPE: u8 = message_type::HEARTBEAT;
}

// =============================================================================
// GENERIC FRAME
// =============================================================================

/// Header + body composition for any [`FrameBody`].
///
/// Layout: `[MitchHeader 16B][B]`. Size = `16 + B::SIZE`.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame<B: FrameBody> {
    /// 16-byte MITCH header (type = `B::MSG_TYPE`, count = 1).
    pub header: MitchHeader,
    /// Fixed-size body.
    pub body: B,
}

#[cfg(feature = "bytemuck")]
unsafe impl<B: FrameBody + bytemuck::Pod> bytemuck::Pod for Frame<B> {}
#[cfg(feature = "bytemuck")]
unsafe impl<B: FrameBody + bytemuck::Zeroable> bytemuck::Zeroable for Frame<B> {}

impl<B: FrameBody> Frame<B> {
    /// Build a frame. `ticks` is a u48 value in 16µs units since 2010-01-01
    /// (see [`crate::timestamp::from_epoch_ms`]).
    #[inline]
    pub fn new(provider_id: u16, ticks: u64, body: B) -> Self {
        Self {
            header: MitchHeader::new(B::MSG_TYPE, provider_id, ticks, 1),
            body,
        }
    }

    /// Raw u48 timestamp (16µs ticks since 2010). Decode with `mitch::timestamp::to_epoch_*`.
    #[inline]
    pub fn timestamp(&self) -> u64 {
        self.header.get_timestamp()
    }

    /// Timestamp as Unix epoch milliseconds.
    #[inline]
    pub fn timestamp_ms(&self) -> i64 {
        crate::timestamp::to_epoch_ms(self.header.get_timestamp())
    }

    /// Provider ID from header.
    #[inline]
    pub fn provider_id(&self) -> u16 {
        self.header.provider_id()
    }

    /// Struct size in bytes.
    #[inline]
    pub const fn size() -> usize {
        message_sizes::HEADER + B::SIZE
    }

    /// Unpack from bytes with validation.
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        let expected = message_sizes::HEADER + B::SIZE;
        if bytes.len() < expected {
            return Err(MitchError::BufferTooSmall {
                expected,
                actual: bytes.len(),
            });
        }
        let frame: Self = unsafe { (bytes.as_ptr() as *const Self).read_unaligned() };
        let mt = frame.header.message_type();
        if mt != B::MSG_TYPE {
            return Err(MitchError::InvalidMessageType(mt));
        }
        Ok(frame)
    }

    /// Unpack without validation (maximum performance).
    ///
    /// # Safety
    /// Caller must ensure `bytes.len() >= 16 + B::SIZE` and data is a valid frame.
    #[inline]
    pub unsafe fn unpack_unchecked(bytes: &[u8]) -> Self {
        (bytes.as_ptr() as *const Self).read_unaligned()
    }
}

// =============================================================================
// CONCRETE FRAME ALIASES + SIZE CONSTANTS
// =============================================================================

/// Tick frame: `[MitchHeader 16B][Tick 32B]` = 48 bytes.
pub type TickFrame = Frame<Tick>;
/// Trade frame: `[MitchHeader 16B][Trade 24B]` = 40 bytes.
pub type TradeFrame = Frame<Trade>;
/// Bar frame: `[MitchHeader 16B][Bar 96B]` = 112 bytes.
pub type BarFrame = Frame<Bar>;
/// Heartbeat frame: `[MitchHeader 16B][Heartbeat 16B]` = 32 bytes.
pub type HeartbeatFrame = Frame<Heartbeat>;

/// Size of a [`TickFrame`] in bytes.
pub const TICK_FRAME_SIZE: usize = message_sizes::HEADER + message_sizes::TICK;
/// Size of a [`TradeFrame`] in bytes.
pub const TRADE_FRAME_SIZE: usize = message_sizes::HEADER + message_sizes::TRADE;
/// Size of a [`BarFrame`] in bytes.
pub const BAR_FRAME_SIZE: usize = message_sizes::HEADER + message_sizes::BAR;
/// Size of a [`HeartbeatFrame`] in bytes.
pub const HEARTBEAT_FRAME_SIZE: usize = message_sizes::HEADER + message_sizes::HEARTBEAT;

const _: () = assert!(core::mem::size_of::<TickFrame>() == TICK_FRAME_SIZE);
const _: () = assert!(core::mem::size_of::<TradeFrame>() == TRADE_FRAME_SIZE);
const _: () = assert!(core::mem::size_of::<BarFrame>() == BAR_FRAME_SIZE);
const _: () = assert!(core::mem::size_of::<HeartbeatFrame>() == HEARTBEAT_FRAME_SIZE);

// =============================================================================
// BODY-SPECIFIC INHERENT IMPLS
// =============================================================================

// `pack()` returns an owned fixed-size byte array. We cannot express
// `[u8; 16 + B::SIZE]` on stable Rust, so each alias gets a tiny inherent
// impl. This is the only duplication, unavoidable until generic_const_exprs
// stabilises.

impl Frame<Tick> {
    /// Pack to bytes (zero-copy transmute).
    pub fn pack(&self) -> [u8; TICK_FRAME_SIZE] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Tick body copy (packed struct cannot yield references).
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
}

impl Frame<Trade> {
    pub fn pack(&self) -> [u8; TRADE_FRAME_SIZE] {
        unsafe { core::mem::transmute(*self) }
    }

    #[inline]
    pub fn trade(&self) -> Trade {
        self.body
    }
}

impl Frame<Bar> {
    pub fn pack(&self) -> [u8; BAR_FRAME_SIZE] {
        unsafe { core::mem::transmute(*self) }
    }

    #[inline]
    pub fn bar(&self) -> Bar {
        self.body
    }
}

impl Frame<Heartbeat> {
    pub fn pack(&self) -> [u8; HEARTBEAT_FRAME_SIZE] {
        unsafe { core::mem::transmute(*self) }
    }

    #[inline]
    pub fn heartbeat(&self) -> Heartbeat {
        self.body
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::OrderSide;
    use crate::timestamp;

    #[test]
    fn frame_sizes() {
        assert_eq!(core::mem::size_of::<TickFrame>(), 48);
        assert_eq!(core::mem::size_of::<TradeFrame>(), 40);
        // BarFrame = 16B header + 96B Bar body = 112 (Bar shrank 128→96B
        // in commit f941ba2; this literal was stale at 144).
        assert_eq!(core::mem::size_of::<BarFrame>(), 112);
        assert_eq!(core::mem::size_of::<HeartbeatFrame>(), 32);
        assert_eq!(TICK_FRAME_SIZE, 48);
        assert_eq!(TRADE_FRAME_SIZE, 40);
        assert_eq!(BAR_FRAME_SIZE, 112);
        assert_eq!(HEARTBEAT_FRAME_SIZE, 32);
    }

    #[test]
    fn tick_frame_round_trip() {
        let ticks = timestamp::from_epoch_ms(1_744_364_200_000);
        let tick = Tick::new_unchecked(1, 100.0, 100.05, 500, 600);
        let frame = TickFrame::new(101, ticks, tick);
        let packed = frame.pack();
        let unpacked = TickFrame::unpack(&packed).unwrap();
        assert_eq!(frame, unpacked);
        assert_eq!(unpacked.provider_id(), 101);
    }

    #[test]
    fn trade_frame_round_trip() {
        let ticks = timestamp::from_epoch_ms(1_744_364_200_000);
        let trade = Trade::new(0x1234, 99.5, 1000, 42, OrderSide::Buy).unwrap();
        let frame = TradeFrame::new(101, ticks, trade);
        let packed = frame.pack();
        let unpacked = TradeFrame::unpack(&packed).unwrap();
        assert_eq!(frame, unpacked);
        assert_eq!(unpacked.provider_id(), 101);
    }

    #[test]
    fn bar_frame_round_trip() {
        let open_mts = timestamp::from_epoch_ms(1_744_372_800_000);
        let close_mts = timestamp::from_epoch_ms(1_744_372_860_000);
        let bar = Bar::new_ohlcv(open_mts, close_mts, 100.0, 105.0, 99.0, 103.0, 1000, 1200, 50);
        let frame = BarFrame::new(101, open_mts, bar);
        let packed = frame.pack();
        let unpacked = BarFrame::unpack(&packed).unwrap();
        assert_eq!(frame, unpacked);
    }

    #[test]
    fn heartbeat_frame_round_trip() {
        let ticks = timestamp::from_epoch_ms(1_744_364_200_000);
        let beat = Heartbeat::ticker(0x1234_5678_9ABC_DEF0, 4242);
        let frame = HeartbeatFrame::new(101, ticks, beat);
        let packed = frame.pack();
        let unpacked = HeartbeatFrame::unpack(&packed).unwrap();
        assert_eq!(frame, unpacked);
        assert_eq!(unpacked.provider_id(), 101);
        let body = unpacked.heartbeat();
        let msg_count = body.msg_count;
        let ticker = body.ticker;
        assert_eq!(msg_count, 4242);
        assert_eq!(ticker, 0x1234_5678_9ABC_DEF0);
    }

    #[test]
    fn heartbeat_frame_feed_wide() {
        let ticks = timestamp::from_epoch_ms(1_744_364_200_000);
        let beat = Heartbeat::feed(0);
        let frame = HeartbeatFrame::new(0, ticks, beat);
        let packed = frame.pack();
        let unpacked = HeartbeatFrame::unpack(&packed).unwrap();
        assert_eq!(frame, unpacked);
        let ticker = unpacked.heartbeat().ticker;
        assert_eq!(ticker, 0);
    }

    #[test]
    fn tick_frame_timestamp() {
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
        let frame = TickFrame::new(1301, 0, tick);
        assert_eq!(frame.provider_id(), 1301);
    }

    #[test]
    fn tick_frame_invalid_type() {
        let tick = Tick::new_unchecked(1, 100.0, 100.05, 0, 0);
        let frame = TickFrame::new(0, 0, tick);
        let mut packed = frame.pack();
        packed[0] = (packed[0] & 0xF0) | 0x0F;
        assert!(TickFrame::unpack(&packed).is_err());
    }

    #[test]
    fn tick_frame_buffer_too_small() {
        let bytes = [0u8; 47];
        assert!(TickFrame::unpack(&bytes).is_err());
    }

    #[test]
    fn trade_frame_rejects_wrong_type() {
        let tick = Tick::new_unchecked(1, 100.0, 100.05, 0, 0);
        let tick_frame = TickFrame::new(0, 0, tick);
        let bytes = tick_frame.pack();
        assert!(TradeFrame::unpack(&bytes[..TRADE_FRAME_SIZE]).is_err());
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
        assert_eq!(bytes.len(), 144);
        let back: &[TickFrame] = bytemuck::cast_slice(bytes);
        assert_eq!(back.len(), 3);
        assert_eq!(back[0], frames[0]);
        assert_eq!(back[2], frames[2]);
    }

    #[cfg(feature = "bytemuck")]
    #[test]
    fn bar_frame_bytemuck_cast() {
        let mts = timestamp::from_epoch_ms(1_700_000_000_000);
        let bar = Bar::new_ohlcv(mts, mts + 1, 100.0, 105.0, 99.0, 103.0, 0, 0, 0);
        let frames: Vec<BarFrame> = (0..5).map(|i| BarFrame::new(101, mts + i, bar)).collect();
        let bytes: &[u8] = bytemuck::cast_slice(&frames);
        assert_eq!(bytes.len(), 5 * BAR_FRAME_SIZE);
        let back: &[BarFrame] = bytemuck::cast_slice(bytes);
        assert_eq!(back.len(), 5);
        assert_eq!(back[4], frames[4]);
    }
}
