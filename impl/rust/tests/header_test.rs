//! Integration tests for the MITCH Header implementation.
//!
//! Tests for:
//! - Header creation with provider_id
//! - Packing and unpacking (serialization/deserialization) roundtrip
//! - Validation logic for message types, counts, and provider IDs
//! - Timestamp handling and 48-bit truncation
//! - Message type code ↔ ASCII mapping

#![allow(clippy::all)]
use mitch::header::*;
use mitch::common::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_new() {
        let header = MitchHeader::new(message_type::TRADE, 101, 1234567890, 5);
        assert_eq!(header.message_type(), message_type::TRADE);
        assert_eq!(header.provider_id(), 101);
        assert_eq!(header.get_timestamp(), 1234567890);
        assert_eq!(header.count, 5);
    }

    #[test]
    fn test_header_pack_unpack() {
        let original = MitchHeader::new(message_type::TICK, 1301, 9876543210, 100);
        let packed = original.pack();
        assert_eq!(packed.len(), 16);

        let unpacked = MitchHeader::unpack(&packed).unwrap();
        assert_eq!(original, unpacked);
        assert_eq!(unpacked.provider_id(), 1301);
        assert_eq!(unpacked.message_type(), message_type::TICK);
    }

    #[test]
    fn test_timestamp_handling() {
        let mut header = MitchHeader::new(message_type::ORDER, 0, 0, 1);

        // Test setting large timestamp (should truncate to 48 bits)
        let large_timestamp = 0xFFFFFFFFFFFFFFFF_u64;
        header.set_timestamp(large_timestamp);
        let retrieved = header.get_timestamp();

        // Should be truncated to 48 bits
        assert_eq!(retrieved, large_timestamp & 0xFFFFFFFFFFFF);
    }

    #[test]
    fn test_invalid_message_type() {
        let result = MitchHeader::new_validated(b'x', 0, 1000, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_count() {
        let result = MitchHeader::new_validated(message_type::TRADE, 0, 1000, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_id_overflow() {
        let result = MitchHeader::new_validated(message_type::TRADE, 4096, 1000, 1);
        assert!(result.is_err()); // 4096 > 4095 (max u12)
    }

    #[test]
    fn test_provider_id_max() {
        let result = MitchHeader::new_validated(message_type::TRADE, 4095, 1000, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().provider_id(), 4095);
    }

    #[test]
    fn test_size_calculation() {
        let header = MitchHeader::new(message_type::TRADE, 0, 1000, 10);
        assert_eq!(header.total_message_size(32), 16 + 10 * 32);
    }

    #[test]
    fn test_message_type_validation() {
        // Test all valid message types
        assert!(MitchHeader::new_validated(message_type::TRADE, 0, 1000, 1).is_ok());
        assert!(MitchHeader::new_validated(message_type::ORDER, 0, 1000, 1).is_ok());
        assert!(MitchHeader::new_validated(message_type::TICK, 0, 1000, 1).is_ok());
        assert!(MitchHeader::new_validated(message_type::INDEX, 0, 1000, 1).is_ok());
        assert!(MitchHeader::new_validated(message_type::ORDER_BOOK, 0, 1000, 1).is_ok());
        assert!(MitchHeader::new_validated(message_type::BAR, 0, 1000, 1).is_ok());

        // Test invalid message type
        assert!(MitchHeader::new_validated(b'z', 0, 1000, 1).is_err());
    }

    #[test]
    fn test_header_size() {
        assert_eq!(core::mem::size_of::<MitchHeader>(), 16);
    }

    #[test]
    fn test_display_format() {
        let header = MitchHeader::new(message_type::TRADE, 101, 123456, 5);
        let display_str = format!("{}", header);
        assert!(display_str.contains("type: 't'"));
        assert!(display_str.contains("provider: 101"));
        assert!(display_str.contains("timestamp: 123456"));
        assert!(display_str.contains("count: 5"));
    }

    #[test]
    fn test_sequence_number() {
        let mut header = MitchHeader::new(message_type::INDEX, 101, 0, 1);
        let seq = header.sequence;
        assert_eq!(seq, 0);
        header.set_sequence(42);
        let seq = header.sequence;
        assert_eq!(seq, 42);

        // Round-trip through pack/unpack
        let packed = header.pack();
        let unpacked = MitchHeader::unpack(&packed).unwrap();
        let seq = unpacked.sequence;
        assert_eq!(seq, 42);
    }

    #[test]
    fn test_message_type_code_mapping() {
        // Verify all types round-trip through code mapping
        for &mt in &[message_type::TRADE, message_type::ORDER, message_type::TICK,
                     message_type::INDEX, message_type::ORDER_BOOK, message_type::BAR] {
            let code = msg_type_to_code(mt);
            assert_ne!(code, 0, "code for {:?} should not be 0", mt as char);
            let back = code_to_msg_type(code);
            assert_eq!(back, mt, "round-trip failed for {:?}", mt as char);
        }
    }

    #[test]
    fn test_all_providers_round_trip() {
        // Test a selection of real provider IDs
        for pid in [0u16, 1, 101, 261, 341, 911, 1301, 4095] {
            let header = MitchHeader::new(message_type::TICK, pid, 12345, 1);
            let packed = header.pack();
            let unpacked = MitchHeader::unpack(&packed).unwrap();
            assert_eq!(unpacked.provider_id(), pid, "provider_id mismatch for {}", pid);
            assert_eq!(unpacked.message_type(), message_type::TICK);
        }
    }
}
