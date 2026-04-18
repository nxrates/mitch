//! Integration tests for the Tick message type.
#![allow(clippy::all)]
use mitch::{self, common::*, tick::*};

#[cfg(test)]
mod tests {
    use super::*;
    use mitch::{MitchError, pack_batch, unpack_batch};

    /// Returns a default, valid Tick message for testing.
    fn get_default_tick() -> Tick {
        Tick::new(
            0x0300_6F30_1CD0_0001, // Ticker: FX, EUR/USD, Venue 1
            1.08750,              // bid_price
            1.08752,              // ask_price
            1_000_000,            // bid_volume
            1_200_000,            // ask_volume
        ).unwrap()
    }

    #[test]
    fn test_tick_size() {
        assert_eq!(core::mem::size_of::<Tick>(), message_sizes::TICK);
        assert_eq!(core::mem::size_of::<Tick>(), 32);
    }

    #[test]
    fn test_tick_pack_unpack_roundtrip() {
        let original = get_default_tick();
        let packed = original.pack();
        let unpacked = Tick::unpack(&packed).unwrap();
        assert_eq!(original, unpacked);
    }

    #[test]
    fn test_tick_validation() {
        // Valid
        assert!(get_default_tick().validate().is_ok());

        // Invalid
        assert!(Tick::new(0, 1.0, 1.1, 1, 1).is_err());
        assert!(Tick::new(1, 0.0, 1.1, 1, 1).is_err());
        assert!(Tick::new(1, 1.0, 0.0, 1, 1).is_err());
        assert!(Tick::new(1, 1.1, 1.0, 1, 1).is_err());
    }

    #[test]
    fn test_tick_calculations() {
        let tick = get_default_tick();
        assert_eq!(tick.mid_price(), 1.08751);
        assert!((tick.spread() - 0.00002).abs() < 1e-9);
        assert!((tick.spread_bps() - 0.18390635488529783).abs() < 1e-9);
        assert_eq!(tick.total_volume(), 2_200_000);
        assert!((tick.volume_imbalance() - (1_200_000.0 - 1_000_000.0) / 2_200_000.0).abs() < 1e-9);
    }

    #[test]
    fn test_tick_batch_operations() {
        let tick1 = get_default_tick();
        let mut tick2 = get_default_tick();
        tick2.bid = 1.09000;
        tick2.ask = 1.09002;

        let messages = vec![tick1, tick2];
        let packed = pack_batch(&messages);
        let unpacked: Vec<Tick> = unpack_batch(&packed, 2).unwrap();

        assert_eq!(messages.len(), unpacked.len());
        assert_eq!(messages[0], unpacked[0]);
        assert_eq!(messages[1], unpacked[1]);
    }

    #[test]
    fn test_unpack_error_handling() {
        let packed = get_default_tick().pack();

        let res = Tick::unpack(&packed[..31]);
        assert!(matches!(res, Err(MitchError::BufferTooSmall { .. })));

        let res_batch: Result<Vec<Tick>, _> = unpack_batch(&packed, 2);
        assert!(matches!(res_batch, Err(MitchError::BufferTooSmall { .. })));
    }
}
