//! MITCH unified message header (16 bytes)
//!
//! This module implements the 16-byte MITCH header that prefixes all message types.
//! The header carries message type, provider ID, timestamp, batch count, flags,
//! and a sequence number for gap detection.
//!
//! ## Wire layout (little-endian)
//!
//! ```text
//! Offset | Field          | Size | Type     | Description
//! -------|----------------|------|----------|--------------------------------------------
//! 0      | type_provider  | 2    | u16 LE   | [3:0] = msg_type code, [15:4] = provider_id
//! 2      | timestamp      | 6    | u48 LE   | 16µs ticks since 2010-01-01T00:00:00Z
//! 8      | count          | 1    | u8       | batch entry count (1-255)
//! 9      | flags          | 1    | u8       | [1:0] = version, [7:2] = reserved
//! 10     | sequence       | 2    | u16 LE   | per-stream sequence for gap detection
//! 12     | _reserved      | 4    | [u8; 4]  | reserved (future: CRC32, frag, etc.)
//! ```
//!
//! ## Message type encoding
//!
//! The ASCII message type codes ('t', 'o', 's', 'i', 'b', 'k') are mapped to
//! 4-bit wire codes (1-6) in the low nibble of `type_provider`. Provider ID
//! occupies bits 4..16 (12 bits, max 4095). See [`common::msg_type_to_code`]
//! and [`common::code_to_msg_type`] for the mapping.
//!
//! ## Body alignment
//!
//! The 16-byte header ensures body structs start at offset 16, preserving
//! natural alignment for u64/f64 fields and enabling zero-copy SIMD loads.

use crate::common::{
    message_sizes, MitchError,
    validate_message_type, msg_type_to_code, code_to_msg_type,
    validate_message_type_code,
};

/// MITCH unified message header (16 bytes)
///
/// Wire format: `[type_provider:u16][timestamp:u48][count:u8][flags:u8][sequence:u16][reserved:4B]`
///
/// Resolution: 16 microseconds. Overflow: ~2162 (142 years from epoch).
/// Encode/decode via `mitch::timestamp::{from_epoch_us, to_epoch_us}` etc.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct MitchHeader {
    /// u16 LE: low 4 bits = message type code (1-6), bits 4..16 = provider_id (0-4095)
    pub type_provider: u16,
    /// u48: 16µs ticks since 2010-01-01T00:00:00Z (6 bytes, LE)
    pub timestamp: [u8; 6],
    /// u8: number of body entries (1-255)
    pub count: u8,
    /// u8: flags - [1:0] = version (0), [7:2] = reserved
    pub flags: u8,
    /// u16 LE: per-stream sequence number for gap detection
    pub sequence: u16,
    /// Reserved for future use (CRC32, fragmentation, etc.)
    pub _reserved: [u8; 4],
}

impl MitchHeader {
    /// Create new header.
    ///
    /// # Arguments
    /// * `message_type` - ASCII character for message type ('t', 'o', 's', 'b', 'i', 'k')
    /// * `provider_id` - Data source provider ID (0-4095)
    /// * `timestamp` - 16µs ticks since 2010-01-01 (see `mitch::timestamp`)
    /// * `count` - Number of body entries (1-255)
    ///
    /// # Panics
    /// Panics if message_type is invalid, count is 0, or provider_id > 4095
    pub fn new(message_type: u8, provider_id: u16, timestamp: u64, count: u8) -> Self {
        validate_message_type(message_type).expect("Invalid message type");
        assert!(count > 0, "Count must be greater than 0");
        assert!(provider_id <= 0x0FFF, "Provider ID must fit in 12 bits (max 4095)");

        let code = msg_type_to_code(message_type);
        let tp = (code as u16 & 0x0F) | (provider_id << 4);

        let mut ts_bytes = [0u8; 6];
        let ts_le_bytes = timestamp.to_le_bytes();
        ts_bytes.copy_from_slice(&ts_le_bytes[0..6]);

        Self {
            type_provider: tp,
            timestamp: ts_bytes,
            count,
            flags: 0,
            sequence: 0,
            _reserved: [0; 4],
        }
    }

    /// Create new header with validated inputs.
    ///
    /// # Arguments
    /// * `message_type` - ASCII character for message type
    /// * `provider_id` - Data source provider ID (0-4095)
    /// * `timestamp` - 16µs ticks since 2010-01-01 (see `mitch::timestamp`)
    /// * `count` - Number of body entries
    ///
    /// # Returns
    /// Result containing new MitchHeader or error
    pub fn new_validated(message_type: u8, provider_id: u16, timestamp: u64, count: u8) -> Result<Self, MitchError> {
        validate_message_type(message_type)?;

        if count == 0 {
            return Err(MitchError::InvalidData("Count must be greater than 0".to_string()));
        }
        if provider_id > 0x0FFF {
            return Err(MitchError::InvalidData("Provider ID must fit in 12 bits (max 4095)".to_string()));
        }

        let code = msg_type_to_code(message_type);
        let tp = (code as u16 & 0x0F) | (provider_id << 4);

        let mut ts_bytes = [0u8; 6];
        let ts_le_bytes = timestamp.to_le_bytes();
        ts_bytes.copy_from_slice(&ts_le_bytes[0..6]);

        Ok(Self {
            type_provider: tp,
            timestamp: ts_bytes,
            count,
            flags: 0,
            sequence: 0,
            _reserved: [0; 4],
        })
    }

    /// Get message type as ASCII code ('t', 'o', 's', 'i', 'b', 'k').
    #[inline]
    pub fn message_type(&self) -> u8 {
        code_to_msg_type((self.type_provider & 0x0F) as u8)
    }

    /// Get the 4-bit message type wire code (1-6).
    #[inline]
    pub fn message_type_code(&self) -> u8 {
        (self.type_provider & 0x0F) as u8
    }

    /// Get provider ID (12 bits, 0-4095).
    #[inline]
    pub fn provider_id(&self) -> u16 {
        self.type_provider >> 4
    }

    /// Get raw u48 timestamp as u64 (Little-Endian)
    ///
    /// # Returns
    /// 16µs ticks since 2010-01-01. Decode with `mitch::timestamp::to_epoch_*`.
    pub fn get_timestamp(&self) -> u64 {
        let mut ts_bytes = [0u8; 8];
        ts_bytes[0..6].copy_from_slice(&self.timestamp);
        u64::from_le_bytes(ts_bytes)
    }

    /// Set timestamp from u64 (automatically truncates to u48, Little-Endian)
    ///
    /// # Arguments
    /// * `timestamp` - 16µs ticks since 2010-01-01. Encode with `mitch::timestamp::from_epoch_*`.
    pub fn set_timestamp(&mut self, timestamp: u64) {
        let ts_le_bytes = timestamp.to_le_bytes();
        self.timestamp.copy_from_slice(&ts_le_bytes[0..6]);
    }

    /// Set provider ID (12 bits, 0-4095). Preserves message type.
    #[inline]
    pub fn set_provider_id(&mut self, provider_id: u16) {
        debug_assert!(provider_id <= 0x0FFF);
        self.type_provider = (self.type_provider & 0x0F) | (provider_id << 4);
    }

    /// Set sequence number for gap detection.
    #[inline]
    pub fn set_sequence(&mut self, seq: u16) {
        self.sequence = seq;
    }

    /// Get protocol version from flags (bits 0-1).
    #[inline]
    pub fn version(&self) -> u8 {
        self.flags & 0x03
    }

    /// Pack MitchHeader into bytes using raw pointer casting (ultra-fast)
    ///
    /// Safe because MitchHeader is `#[repr(C, packed)]` with only POD types.
    ///
    /// # Returns
    /// 16-byte array containing the packed header
    pub fn pack(&self) -> [u8; message_sizes::HEADER] {
        unsafe {
            let ptr = self as *const Self as *const u8;
            let mut result = [0u8; message_sizes::HEADER];
            core::ptr::copy_nonoverlapping(ptr, result.as_mut_ptr(), message_sizes::HEADER);
            result
        }
    }

    /// Unpack MitchHeader from bytes using raw pointer casting (ultra-fast)
    ///
    /// Validates buffer size and message type code.
    ///
    /// # Arguments
    /// * `bytes` - Byte slice containing header data (>= 16 bytes)
    ///
    /// # Returns
    /// Result containing unpacked MitchHeader or error
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::HEADER {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::HEADER,
                actual: bytes.len(),
            });
        }

        unsafe {
            let ptr = bytes.as_ptr() as *const Self;
            let header = ptr.read_unaligned();

            // Validate the 4-bit message type code
            validate_message_type_code(header.message_type_code())?;
            if header.count == 0 {
                return Err(MitchError::InvalidData("Count cannot be 0".to_string()));
            }

            Ok(header)
        }
    }

    /// Get the total message size including body
    ///
    /// # Arguments
    /// * `body_size` - Size of a single body entry in bytes
    ///
    /// # Returns
    /// Total message size in bytes
    pub fn total_message_size(&self, body_size: usize) -> usize {
        message_sizes::HEADER + (self.count as usize * body_size)
    }

    /// Validate header consistency
    pub fn validate(&self) -> Result<(), MitchError> {
        validate_message_type_code(self.message_type_code())?;

        if self.count == 0 {
            return Err(MitchError::InvalidData("Count must be greater than 0".to_string()));
        }

        if self.provider_id() > 0x0FFF {
            return Err(MitchError::InvalidData("Provider ID exceeds 12 bits".to_string()));
        }

        Ok(())
    }

    /// Get message type as character
    pub fn message_type_char(&self) -> char {
        self.message_type() as char
    }
}

impl Default for MitchHeader {
    fn default() -> Self {
        let code = msg_type_to_code(b's'); // Default to tick message
        Self {
            type_provider: code as u16, // provider_id = 0
            timestamp: [0; 6],
            count: 1,
            flags: 0,
            sequence: 0,
            _reserved: [0; 4],
        }
    }
}

// =============================================================================
// DISPLAY IMPLEMENTATIONS
// =============================================================================

impl core::fmt::Display for MitchHeader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Copy fields out of packed struct to avoid unaligned references
        let seq = self.sequence;
        let count = self.count;
        write!(
            f,
            "MitchHeader {{ type: '{}', provider: {}, timestamp: {}, count: {}, seq: {} }}",
            self.message_type_char(),
            self.provider_id(),
            self.get_timestamp(),
            count,
            seq,
        )
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

// Compile-time size assertion
const _: () = assert!(core::mem::size_of::<MitchHeader>() == message_sizes::HEADER, "MitchHeader must be exactly 16 bytes");
