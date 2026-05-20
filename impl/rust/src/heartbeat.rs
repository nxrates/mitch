//! MITCH Heartbeat message implementation (16 bytes body, 32 bytes frame).
//!
//! Heartbeats prove a producer (forwarder, aggregator, publisher) is alive and
//! let consumers spot stale feeds and quantify drop rates via the emitted
//! message counter. The body is deliberately tiny and payload-agnostic: one
//! heartbeat frame shape fits every producer (UDP multicast, TCP unicast,
//! WebSocket fan-out).
//!
//! ## Frame shape
//!
//! `[MitchHeader 16B][Heartbeat 16B]` = 32B on the wire.
//!
//! Use `ticker = 0` for a feed-wide heartbeat (producer-level liveness) and a
//! nonzero `ticker` for per-ticker heartbeats (used by the aggregator to keep
//! slow-moving symbols visible without forcing a full snapshot).
//!
//! ## Counter semantics
//!
//! `msg_count` is the number of data frames the producer has emitted in the
//! scope implied by `ticker` since the previous heartbeat. Consumers diff
//! successive counters to detect gaps: the sequence in `MitchHeader` tracks
//! heartbeat-to-heartbeat gaps, `msg_count` tracks data-frame gaps between
//! them. The counter wraps naturally at `u32::MAX`.

use crate::body::MitchBody;
use crate::common::{message_sizes, MitchError};

/// Heartbeat body (16 bytes).
///
/// ## Wire layout (little-endian)
///
/// ```text
/// Offset | Field     | Size | Type    | Description
/// -------|-----------|------|---------|-------------------------------------
/// 0      | ticker    | 8    | u64 LE  | Ticker (0 = feed-wide, else per-symbol)
/// 8      | msg_count | 4    | u32 LE  | Data frames emitted since last beat
/// 12     | _pad      | 4    | [u8; 4] | Reserved padding for alignment
/// ```
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Heartbeat {
    /// Ticker this heartbeat refers to. `0` means the heartbeat applies to the
    /// full producer feed rather than a single symbol.
    pub ticker: u64,
    /// Data frames the producer has emitted in this scope since the prior
    /// heartbeat. Wraps at `u32::MAX`.
    pub msg_count: u32,
    /// Reserved padding. Keeps the body 8-byte aligned and leaves room for a
    /// future flags field without re-sizing the wire frame.
    pub _pad: [u8; 4],
}

impl Heartbeat {
    /// Feed-wide heartbeat: `ticker = 0`, carrying the producer's rolling
    /// message count since the previous beat.
    #[inline]
    pub const fn feed(msg_count: u32) -> Self {
        Self { ticker: 0, msg_count, _pad: [0; 4] }
    }

    /// Per-ticker heartbeat.
    #[inline]
    pub const fn ticker(ticker: u64, msg_count: u32) -> Self {
        Self { ticker, msg_count, _pad: [0; 4] }
    }

    /// Pack into raw bytes (zero-copy transmute).
    pub fn pack(&self) -> [u8; message_sizes::HEARTBEAT] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Unpack from bytes. Returns `BufferTooSmall` if `bytes.len() < 16`.
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::HEARTBEAT {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::HEARTBEAT,
                actual: bytes.len(),
            });
        }
        unsafe {
            let ptr = bytes.as_ptr() as *const Self;
            Ok(ptr.read_unaligned())
        }
    }

    /// Body size in bytes (16).
    pub const fn size() -> usize {
        message_sizes::HEARTBEAT
    }
}

// SAFETY: Heartbeat is `#[repr(C, packed)]` with only POD fields; no padding
// bytes beyond the explicit `_pad` tail, which is initialized to zero.
unsafe impl MitchBody for Heartbeat {
    const SIZE: usize = message_sizes::HEARTBEAT;
}

// Compile-time size assertion
const _: () = assert!(
    core::mem::size_of::<Heartbeat>() == message_sizes::HEARTBEAT,
    "Heartbeat must be exactly 16 bytes"
);
