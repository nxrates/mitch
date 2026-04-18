//! Integration tests for the Trade message type.
//!
//! This file contains tests for:
//! - Correct size of the Trade struct.
//! - Packing and unpacking (serialization/deserialization) roundtrip.
//! - Validation logic for all fields.
//! - Batch operations for packing and unpacking multiple messages.

#![allow(clippy::all)]
use mitch::{self, common::*, trade::*};

#[cfg(test)]
mod tests {
    use super::*;
    use mitch::{MitchError, pack_batch, unpack_batch};

    /// Returns a default, valid Trade message for testing.
    fn get_default_trade() -> Trade {
        Trade::new(
            0x0300_6F30_1CD0_0001, // Ticker: FX, EUR/USD, Venue 1
            1.08750,              // price
            1_000_000,            // qty
            12345,                // trade_id
            OrderSide::Buy,       // side
        ).unwrap()
    }

    #[test]
    fn test_trade_size() {
        assert_eq!(core::mem::size_of::<Trade>(), message_sizes::TRADE);
        assert_eq!(core::mem::size_of::<Trade>(), 24);
    }

    #[test]
    fn test_trade_pack_unpack_roundtrip() {
        let original = get_default_trade();
        let packed = original.pack();
        let unpacked = Trade::unpack(&packed).unwrap();
        assert_eq!(original, unpacked);
    }

    #[test]
    fn test_trade_validation() {
        // --- Valid Trade ---
        let trade = get_default_trade();
        assert!(trade.validate().is_ok());

        // --- Invalid fields ---
        assert!(Trade::new(0, 1.0, 1, 1, OrderSide::Buy).is_err(), "Zero ticker");
        assert!(Trade::new(1, 0.0, 1, 1, OrderSide::Buy).is_err(), "Zero price");
        assert!(Trade::new(1, -1.0, 1, 1, OrderSide::Buy).is_err(), "Negative price");
        assert!(Trade::new(1, 1.0, 0, 1, OrderSide::Buy).is_err(), "Zero qty");
        assert!(Trade::new(1, 1.0, 1, 0, OrderSide::Buy).is_err(), "Zero trade ID");
    }

    #[test]
    fn test_trade_helpers() {
        let buy_trade = get_default_trade();
        let mut sell_trade = get_default_trade();
        sell_trade.side = OrderSide::Sell as u8;

        assert!(buy_trade.is_buy());
        assert!(!buy_trade.is_sell());
        assert!(!sell_trade.is_buy());
        assert!(sell_trade.is_sell());
        assert_eq!(buy_trade.notional_value(), 1_087_500.0);
    }

    #[test]
    fn test_trade_id_u24_encoding() {
        // Verify trade_id round-trips through the u24 encoding
        let trade = get_default_trade();
        assert_eq!(trade.get_trade_id(), 12345);

        // Max u24 value
        let max_trade = Trade::new(1, 1.0, 1, 0xFF_FFFF, OrderSide::Buy).unwrap();
        assert_eq!(max_trade.get_trade_id(), 16_777_215);
        assert_eq!(max_trade.trade_id, [0xFF, 0xFF, 0xFF]);

        // Verify little-endian byte order: 0x030201
        let trade2 = Trade::new(1, 1.0, 1, 0x03_0201, OrderSide::Buy).unwrap();
        assert_eq!(trade2.trade_id, [0x01, 0x02, 0x03]);
        assert_eq!(trade2.get_trade_id(), 0x03_0201);
    }

    #[test]
    fn test_trade_set_trade_id() {
        let mut trade = get_default_trade();
        trade.set_trade_id(999);
        assert_eq!(trade.get_trade_id(), 999);
        trade.set_trade_id(0xFF_FFFF);
        assert_eq!(trade.get_trade_id(), 16_777_215);
    }

    #[test]
    fn test_trade_batch_operations() {
        let trade1 = get_default_trade();
        let mut trade2 = get_default_trade();
        trade2.set_trade_id(54321);

        let messages = vec![trade1, trade2];
        let packed = pack_batch(&messages);
        let unpacked: Vec<Trade> = unpack_batch(&packed, 2).unwrap();

        assert_eq!(messages.len(), unpacked.len());
        assert_eq!(messages[0], unpacked[0]);
        assert_eq!(messages[1], unpacked[1]);
    }

    #[test]
    fn test_unpack_error_handling() {
        let original = get_default_trade();
        let packed = original.pack();

        // Buffer too small for single message
        let res = Trade::unpack(&packed[..23]);
        assert!(matches!(
            res,
            Err(MitchError::BufferTooSmall {
                expected: 24,
                actual: 23
            })
        ));

        // Buffer too small for batch
        let res_batch: Result<Vec<Trade>, _> = unpack_batch(&packed, 2);
         assert!(matches!(
            res_batch,
            Err(MitchError::BufferTooSmall {
                expected: 48,
                actual: 24
            })
        ));
    }
}
