//! Integration tests for the Channel ID utilities.
//!
//! This file contains tests for:
//! - Channel ID creation and field extraction
//! - Packing and unpacking (serialization/deserialization) roundtrip
//! - Validation logic for message types
//! - 16-bit provider ID support as per specification

#![allow(clippy::all)]
use mitch::channel::*;
use mitch::common::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let channel = ChannelId::new(101, 's');
        assert_eq!(channel.provider(), 101);
        assert_eq!(channel.msg_type(), 's');
        assert_eq!(channel.padding(), 0);
        assert_eq!(channel.raw, (101u32 << 16) | (('s' as u32) << 8));
    }

    #[test]
    fn test_16bit_provider_support() {
        let channel = ChannelId::new(65535, 't'); // Max 16-bit value
        assert_eq!(channel.provider(), 65535);
        assert_eq!(channel.msg_type(), 't');
        assert_eq!(channel.padding(), 0);
    }

    #[test]
    fn test_pack_unpack() {
        let original = ChannelId::new(101, 's');
        let packed = original.pack();
        let unpacked = ChannelId::unpack(&packed).unwrap();
        assert_eq!(original.raw, unpacked.raw);
        assert_eq!(original.provider(), unpacked.provider());
        assert_eq!(original.msg_type(), unpacked.msg_type());
    }

    #[test]
    fn test_validation() {
        let valid = ChannelId::new(1, 't');
        assert!(valid.validate().is_ok());

        let valid_order = ChannelId::new(1, 'o');
        assert!(valid_order.validate().is_ok());

        let valid_tick = ChannelId::new(1, 's');
        assert!(valid_tick.validate().is_ok());

        let valid_index = ChannelId::new(1, 'i');
        assert!(valid_index.validate().is_ok());

        let valid_book = ChannelId::new(1, 'b');
        assert!(valid_book.validate().is_ok());

        let invalid = ChannelId::new(1, 'x');
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_display_format() {
        let channel = ChannelId::new(101, 's');
        let display_str = format!("{}", channel);
        assert!(display_str.contains("provider=101"));
        assert!(display_str.contains("type='s'"));
    }

    #[test]
    fn test_buffer_too_small() {
        let small_buffer = [0u8; 2]; // Only 2 bytes, need 4
        let result = ChannelId::unpack(&small_buffer);
        assert!(result.is_err());

        if let Err(MitchError::BufferTooSmall { expected, actual }) = result {
            assert_eq!(expected, 4);
            assert_eq!(actual, 2);
        } else {
            panic!("Expected BufferTooSmall error");
        }
    }

    #[test]
    fn test_field_extraction() {
        let channel = ChannelId::new(12345, 'i');
        assert_eq!(channel.provider(), 12345);
        assert_eq!(channel.msg_type(), 'i');
        assert_eq!(channel.padding(), 0);

        // Test raw value construction
        let expected_raw = (12345u32 << 16) | (('i' as u32) << 8);
        assert_eq!(channel.raw, expected_raw);
    }
}
