//! Channel ID Utilities
//!
//! Provides 4-byte channel identifiers for efficient pub/sub routing.
//! Channel ID = (provider_u16 << 16) | (msg_type_u8 << 8) | padding_u8
//!
//! Used for topic-based filtering in messaging systems.

use crate::common::*;

/// Channel ID structure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelId {
    /// Raw 32-bit channel identifier
    pub raw: u32,
}

impl ChannelId {
    /// Create new channel ID
    ///
    /// # Arguments
    /// * `provider` - Market provider ID (0-65535, 16-bit)
    /// * `msg_type` - Message type ASCII char ('t','o','s','i','b')
    pub fn new(provider: u16, msg_type: char) -> Self {
        let type_byte = msg_type as u8;
        let raw = ((provider as u32) << 16) | ((type_byte as u32) << 8);
        Self { raw }
    }

    /// Extract provider ID (16-bit)
    pub fn provider(&self) -> u16 {
        (self.raw >> 16) as u16
    }

    /// Extract message type char
    pub fn msg_type(&self) -> char {
        let byte = ((self.raw >> 8) & 0xFF) as u8;
        byte as char
    }

    /// Extract padding byte
    pub fn padding(&self) -> u8 {
        (self.raw & 0xFF) as u8
    }

    /// Pack to 4 bytes (Little-Endian)
    pub fn pack(&self) -> [u8; 4] {
        self.raw.to_le_bytes()
    }

    /// Unpack from bytes
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < 4 {
            return Err(MitchError::BufferTooSmall { expected: 4, actual: bytes.len() });
        }
        unsafe {
            let ptr = bytes.as_ptr() as *const u32;
            let raw = ptr.read_unaligned().to_le();
            Ok(Self { raw })
        }
    }

    /// Validate channel ID
    pub fn validate(&self) -> Result<(), MitchError> {
        let msg_byte = self.msg_type() as u8;
        if !matches!(msg_byte, b't' | b'o' | b's' | b'i' | b'b') {
            return Err(MitchError::InvalidFieldValue("msg_type".into()));
        }
        Ok(())
    }
}

impl core::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Channel(provider={}, type='{}')", self.provider(), self.msg_type())
    }
}
