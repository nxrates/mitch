//! Generic zero-copy batch serialization for MITCH bodies.
//!
//! All MITCH body types (Tick, Trade, Order, Index, OrderBook, Bar) share the
//! same wire pattern: fixed-size `#[repr(C, packed)]` structs serialized via
//! bulk memcpy. The [`MitchBody`] trait captures that contract, and the free
//! functions [`pack_batch`], [`unpack_batch`], [`unpack_all`], [`write_batch`]
//! operate over any implementer.
//!
//! Per-type inherent `pack`/`unpack` methods on the concrete types remain for
//! ergonomic single-message access and returning a fixed-size `[u8; N]`.

use crate::common::MitchError;
use core::ptr;

/// Marker + size contract for MITCH wire body types.
///
/// # Safety
///
/// Implementers MUST be `#[repr(C, packed)]` (or otherwise byte-stable) and
/// `Copy`, with a fixed on-wire size equal to `SIZE` and no padding bytes.
/// Batch helpers transmute slices of `T` to/from `[u8]` via `copy_nonoverlapping`;
/// any padding would leak uninitialized memory (UB / infoleak).
pub unsafe trait MitchBody: Sized + Copy {
    /// On-wire byte size of one body.
    const SIZE: usize;
}

/// Pack a slice of bodies into a contiguous byte vector (bulk memcpy).
#[inline]
pub fn pack_batch<T: MitchBody>(items: &[T]) -> Vec<u8> {
    let total = items.len() * T::SIZE;
    let mut buf = Vec::with_capacity(total);
    unsafe {
        ptr::copy_nonoverlapping(items.as_ptr() as *const u8, buf.as_mut_ptr(), total);
        buf.set_len(total);
    }
    buf
}

/// Unpack `count` bodies from a byte slice (zero-copy `read_unaligned`).
#[inline]
pub fn unpack_batch<T: MitchBody>(bytes: &[u8], count: usize) -> Result<Vec<T>, MitchError> {
    let expected = count * T::SIZE;
    if bytes.len() < expected {
        return Err(MitchError::BufferTooSmall {
            expected,
            actual: bytes.len(),
        });
    }
    let mut out = Vec::with_capacity(count);
    unsafe {
        let p = bytes.as_ptr() as *const T;
        for i in 0..count {
            out.push(p.add(i).read_unaligned());
        }
    }
    Ok(out)
}

/// Unpack all bodies from a byte slice, inferring count from length.
#[inline]
pub fn unpack_all<T: MitchBody>(bytes: &[u8]) -> Result<Vec<T>, MitchError> {
    if bytes.len() % T::SIZE != 0 {
        return Err(MitchError::InvalidData(format!(
            "buffer length {} is not a multiple of body size {}",
            bytes.len(),
            T::SIZE,
        )));
    }
    unpack_batch(bytes, bytes.len() / T::SIZE)
}

