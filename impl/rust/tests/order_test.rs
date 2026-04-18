//! Integration tests for the Order message type.
#![allow(clippy::all)]
use mitch::{self, common::*, order::*};

#[cfg(test)]
mod tests {
    use super::*;
    use mitch::{MitchError, pack_batch, unpack_batch};

    /// Returns a default, valid Order message for testing.
    fn get_default_order() -> Order {
        Order::new(
            0x0300_6F30_1CD0_0001, // Ticker: FX, EUR/USD, Venue 1
            54321,                // order_id
            1.08750,              // price
            1_000_000,            // quantity
            OrderType::Limit,     // order_type
            OrderSide::Buy,       // side
            1700000000123,        // expiry (in ms)
        ).unwrap()
    }

    #[test]
    fn test_order_size() {
        assert_eq!(core::mem::size_of::<Order>(), message_sizes::ORDER);
        assert_eq!(core::mem::size_of::<Order>(), 32);
    }

    #[test]
    fn test_order_pack_unpack_roundtrip() {
        let original = get_default_order();
        let packed = original.pack();
        let unpacked = Order::unpack(&packed).unwrap();
        assert_eq!(original, unpacked);
    }

    #[test]
    fn test_order_validation() {
        // Valid
        assert!(get_default_order().validate().is_ok());

        // Invalid
        assert!(Order::new(0, 1, 1.0, 1, OrderType::Limit, OrderSide::Buy, 0).is_err());
        assert!(Order::new(1, 0, 1.0, 1, OrderType::Limit, OrderSide::Buy, 0).is_err());
        assert!(Order::new(1, 1, 0.0, 1, OrderType::Limit, OrderSide::Buy, 0).is_err());
        assert!(Order::new(1, 1, 1.0, 0, OrderType::Limit, OrderSide::Buy, 0).is_err());

        // Cancel order doesn't require price/quantity validation
        let cancel_order = Order::new(1, 1, 0.0, 0, OrderType::Cancel, OrderSide::Buy, 0).unwrap();
        assert!(cancel_order.validate().is_ok());
    }

    #[test]
    fn test_type_and_side_handling() {
        let order = get_default_order();
        assert_eq!(order.get_order_type(), OrderType::Limit);
        assert_eq!(order.get_order_side(), OrderSide::Buy);

        let market_sell = Order::new(1,1,1.0,1, OrderType::Market, OrderSide::Sell, 0).unwrap();
        assert_eq!(market_sell.get_order_type(), OrderType::Market);
        assert_eq!(market_sell.get_order_side(), OrderSide::Sell);
    }

    #[test]
    fn test_expiry_handling() {
        let mut order = get_default_order();
        assert_eq!(order.get_expiry(), 1700000000123);
        assert!(!order.is_gtc());
        assert!(order.is_expired(1700000000124));
        assert!(!order.is_expired(1700000000122));

        order.set_expiry(0);
        assert!(order.is_gtc());
        assert!(!order.is_expired(u64::MAX));
    }

    #[test]
    fn test_order_batch_operations() {
        let order1 = get_default_order();
        let mut order2 = get_default_order();
        order2.order_id = 12345;

        let messages = vec![order1, order2];
        let packed = pack_batch(&messages);
        let unpacked: Vec<Order> = unpack_batch(&packed, 2).unwrap();

        assert_eq!(messages.len(), unpacked.len());
        assert_eq!(messages[0], unpacked[0]);
        assert_eq!(messages[1], unpacked[1]);
    }

    #[test]
    fn test_unpack_error_handling() {
        let packed = get_default_order().pack();

        let res = Order::unpack(&packed[..31]);
        assert!(matches!(res, Err(MitchError::BufferTooSmall { .. })));

        let res_batch: Result<Vec<Order>, _> = unpack_batch(&packed, 2);
        assert!(matches!(res_batch, Err(MitchError::BufferTooSmall { .. })));
    }

    // Additional order tests moved from src/order.rs

    #[test]
    fn test_order_new() {
        let order = Order::new(
            0x123456789ABCDEF0,
            12345,
            100.50,
            1000,
            OrderType::Limit,
            OrderSide::Buy,
            1640995200000 // Unix timestamp
        ).unwrap();

        // Copy individual fields to avoid packed field access issues
        let ticker = order.ticker;
        let order_id = order.order_id;
        let price = order.price;
        let qty = order.qty;

        assert_eq!(ticker, 0x123456789ABCDEF0);
        assert_eq!(order_id, 12345);
        assert_eq!(price, 100.50);
        assert_eq!(qty, 1000);
        assert_eq!(order.get_order_type(), OrderType::Limit);
        assert_eq!(order.get_order_side(), OrderSide::Buy);
        assert_eq!(order.get_expiry(), 1640995200000);
    }

    #[test]
    fn test_order_pack_unpack() {
        let original = Order::new(
            0x123456789ABCDEF0,
            12345,
            100.50,
            1000,
            OrderType::Limit,
            OrderSide::Buy,
            1700000000123456,
        ).unwrap();

        let packed = original.pack();
        let unpacked = Order::unpack(&packed).unwrap();

        // Copy structs to avoid packed field access issues
        let original_copy = original;
        let unpacked_copy = unpacked;
        assert_eq!(original_copy, unpacked_copy);
    }

    #[test]
    fn test_type_and_side_encoding() {
        let order = Order::new(0x1, 1, 100.0, 100, OrderType::Limit, OrderSide::Sell, 0).unwrap();

        assert_eq!(order.get_order_type(), OrderType::Limit);
        assert_eq!(order.get_order_side(), OrderSide::Sell);

        // Verify bit encoding: Limit = 1, Sell = 1
        // Expected: (1 << 1) | 1 = 3
        assert_eq!(order.type_and_side, 3);
    }

    #[test]
    fn test_expiry_handling_alt() {
        let mut order = Order::new(0x1, 1, 100.0, 100, OrderType::Market, OrderSide::Buy, 0).unwrap();

        // Test GTC (expiry = 0)
        assert!(order.is_gtc());
        assert!(!order.is_expired(999999999999));

        // Test expiry setting
        order.set_expiry(1000000);
        assert_eq!(order.get_expiry(), 1000000);
        assert!(!order.is_gtc());
        assert!(order.is_expired(1000001));
        assert!(!order.is_expired(999999));
    }

    #[test]
    fn test_order_validation_alt() {
        // Valid order
        let valid = Order::new(0x1, 1, 100.0, 500, OrderType::Limit, OrderSide::Buy, 0);
        assert!(valid.is_ok());

        // Invalid order ID
        let invalid_id = Order::new(0x1, 0, 100.0, 500, OrderType::Limit, OrderSide::Buy, 0);
        assert!(invalid_id.is_err());

        // Invalid price
        let invalid_price = Order::new(0x1, 1, -100.0, 500, OrderType::Limit, OrderSide::Buy, 0);
        assert!(invalid_price.is_err());

        // Invalid quantity
        let invalid_quantity = Order::new(0x1, 1, 100.0, 0, OrderType::Limit, OrderSide::Buy, 0);
        assert!(invalid_quantity.is_err());
    }

    #[test]
    fn test_order_helpers() {
        let buy_order = Order::new(0x1, 1, 100.0, 10, OrderType::Limit, OrderSide::Buy, 0).unwrap();
        let sell_order = Order::new(0x1, 2, 100.0, 10, OrderType::Market, OrderSide::Sell, 0).unwrap();

        assert!(buy_order.is_buy());
        assert!(!buy_order.is_sell());
        assert!(sell_order.is_sell());
        assert!(!sell_order.is_buy());

        assert_eq!(buy_order.notional_value(), 1000.0);
    }

    #[test]
    fn test_batch_operations() {
        let orders = vec![
            Order::new(0x1, 1, 100.0, 10, OrderType::Limit, OrderSide::Buy, 0).unwrap(),
            Order::new(0x2, 2, 200.0, 20, OrderType::Market, OrderSide::Sell, 1000).unwrap(),
        ];

        let packed = pack_batch(&orders);
        assert_eq!(packed.len(), 64); // 2 * 32 bytes

        let unpacked: Vec<Order> = unpack_batch(&packed, 2).unwrap();
        assert_eq!(orders, unpacked);
    }

    #[test]
    fn test_48bit_expiry_truncation() {
        let mut order = Order::new(0x1, 1, 100.0, 100, OrderType::Market, OrderSide::Buy, 0).unwrap();

        // Test setting large expiry (should truncate to 48 bits)
        let large_expiry = 0xFFFFFFFFFFFFFFFF_u64;
        order.set_expiry(large_expiry);
        let retrieved = order.get_expiry();

        // Should be truncated to 48 bits
        assert_eq!(retrieved, large_expiry & 0xFFFFFFFFFFFF);
    }

    #[test]
    fn test_size_constant() {
        assert_eq!(Order::size(), 32);
    }
}
