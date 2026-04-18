use mitch::*;

#[test]
fn test_message_size_calculation() {
    // Trade messages (16B header + 24B body)
    assert_eq!(calculate_message_size(message_type::TRADE, 1).unwrap(), 40);
    assert_eq!(calculate_message_size(message_type::TRADE, 10).unwrap(), 256);

    // Order messages (16B header + 32B body)
    assert_eq!(calculate_message_size(message_type::ORDER, 1).unwrap(), 48);
    assert_eq!(calculate_message_size(message_type::ORDER, 5).unwrap(), 176);

    // Tick messages (16B header + 32B body)
    assert_eq!(calculate_message_size(message_type::TICK, 1).unwrap(), 48);
    assert_eq!(calculate_message_size(message_type::TICK, 3).unwrap(), 112);
}

#[test]
fn test_buffer_validation() {
    // Create a valid trade message
    let trade = Trade::new(0x1, 100.0, 1000, 1, OrderSide::Buy).unwrap();
    let header = MitchHeader::new(message_type::TRADE, 0, 123456, 1);

    let mut buffer = Vec::new();
    buffer.extend_from_slice(&header.pack());
    buffer.extend_from_slice(&trade.pack());

    let (msg_type, count) = validate_message_buffer(&buffer).unwrap();
    assert_eq!(msg_type, message_type::TRADE);
    assert_eq!(count, 1);
}

#[test]
fn test_invalid_message_type() {
    let result = calculate_message_size(b'x', 1);
    assert!(result.is_err());
}

#[test]
fn test_version_constants() {
    assert!(!MITCH_VERSION.is_empty());
    assert!(!LIB_VERSION.is_empty());
    assert!(!BUILD_INFO.is_empty());
}
