//! Integration tests for the Index message type (40 bytes).
//!
//! Tests for: size, pack/unpack roundtrip, validation, derived calculations, batch ops.

#![allow(clippy::all)]
use mitch::{self, common::*, index::*};

#[cfg(test)]
mod tests {
    use super::*;
    use mitch::{MitchError, pack_batch, unpack_batch};

    /// Returns a default, valid Index message for testing.
    fn get_default_index() -> Index {
        Index::new(
            0x0A00_6F30_1CD0_0001,     // ticker
            1.08740,                   // bid
            1.08760,                   // ask
            14,                        // ci (micro basis points)
            1_000_000,                 // vbid
            1_200_000,                 // vask
            15,                        // tick_count
            9,                         // confidence
            9,                         // accepted
            1,                         // rejected
        )
    }

    #[test]
    fn test_index_size() {
        assert_eq!(core::mem::size_of::<Index>(), message_sizes::INDEX);
        assert_eq!(core::mem::size_of::<Index>(), 40);
    }

    #[test]
    fn test_index_pack_unpack_roundtrip() {
        let original = get_default_index();
        let packed = original.pack();
        let unpacked = Index::unpack(&packed).unwrap();
        assert_eq!(original, unpacked);
    }

    #[test]
    fn test_index_validation() {
        let mut index = get_default_index();
        assert!(index.validate().is_ok(), "Default index should be valid");

        // Invalid Ticker
        index.ticker = 0;
        assert!(index.validate().is_err());
        index.ticker = get_default_index().ticker;

        // Invalid Bid Price
        index.bid = 0.0;
        assert!(index.validate().is_err());
        index.bid = -1.0;
        assert!(index.validate().is_err());
        index.bid = get_default_index().bid;

        // Invalid Ask Price
        index.ask = 0.0;
        assert!(index.validate().is_err());
        index.ask = -1.0;
        assert!(index.validate().is_err());
        index.ask = get_default_index().ask;

        // Ask < Bid (crossed market)
        index.ask = 1.08700;
        assert!(index.validate().is_err());
        index.ask = get_default_index().ask;

        // Logical inconsistency: confidence without accepted providers
        index.accepted = 0;
        index.confidence = 1;
        assert!(index.validate().is_err());
    }

    #[test]
    fn test_index_mid_derived() {
        let index = get_default_index();
        let expected_mid = (1.08740 + 1.08760) / 2.0;
        assert!((index.mid() - expected_mid).abs() < 1e-15);
    }

    #[test]
    fn test_index_derived_calculations() {
        let index = get_default_index();

        // Spread = ask - bid
        assert!((index.spread() - 0.00020).abs() < 1e-10);

        // Spread bps
        let expected_bps = 0.00020 / index.mid() * 10000.0;
        assert!((index.spread_bps() - expected_bps).abs() < 1e-6);

        // Volume imbalance
        let expected_imbalance = (1_200_000.0 - 1_000_000.0) / (1_200_000.0 + 1_000_000.0);
        assert!((index.volume_imbalance() - expected_imbalance).abs() < 1e-10);

        // CI price conversion: sqrt-compressed u16 -> ubp -> price.
        // ci = 14 (wire) -> ci_ubp = (14/16)^2 = 0.765625 -> price = mid * ci_ubp / 1e8
        let ci_scale: f64 = 16.0;
        let x = 14.0_f64 / ci_scale;
        let expected_ci = index.mid() * (x * x) / 1e8;
        assert!((index.ci_price() - expected_ci).abs() < 1e-15);
    }

    #[test]
    fn test_index_batch_operations() {
        let index1 = get_default_index();
        let mut index2 = get_default_index();
        index2.ticker = 0x0A00_6F30_1CD0_0002;
        index2.bid = 1.08990;
        index2.ask = 1.09010;

        let messages = vec![index1, index2];
        let packed = pack_batch(&messages);
        let unpacked: Vec<Index> = unpack_batch(&packed, 2).unwrap();

        assert_eq!(messages.len(), unpacked.len());
        assert_eq!(messages[0], unpacked[0]);
        assert_eq!(messages[1], unpacked[1]);
    }

    #[test]
    fn test_unpack_error_handling() {
        let original = get_default_index();
        let packed = original.pack();

        // Buffer too small
        let res = Index::unpack(&packed[..39]);
        assert!(matches!(
            res,
            Err(MitchError::BufferTooSmall {
                expected: 40,
                actual: 39
            })
        ));

        // Batch buffer too small
        let res_batch: Result<Vec<Index>, _> = unpack_batch(&packed, 2);
        assert!(matches!(
            res_batch,
            Err(MitchError::BufferTooSmall {
                expected: 80,
                actual: 40
            })
        ));
    }
}
