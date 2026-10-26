// tests/constants_test.rs

#[cfg(test)]
mod constants_tests {
    use mitch::constants::*;

    #[test]
    fn broker_aliases_resolve() {
        for (key, id) in [("icmarkets", 651), ("IC Markets", 651), ("pepperstone", 961)] {
            assert_eq!(resolve_market_providers(key).map(|e| e.id), Some(id), "{key}");
        }
    }

    #[test]
    fn test_asset_class_ids() {
        assert_eq!(AssetClass::EQ as u64, 0);
        assert_eq!(AssetClass::FX as u64, 3);
        assert_eq!(AssetClass::CR as u64, 6);
    }

    #[test]
    fn test_instrument_type_ids() {
        assert_eq!(InstrumentType::SPOT as u64, 0);
        assert_eq!(InstrumentType::FUT as u64, 1);
        assert_eq!(InstrumentType::PERP as u64, 4);
    }

    #[test]
    fn test_bins_hashmap() {
        // Test Lingaussian Bins
        let lingaussian_bins = BINS.get(&BinAggregator::DEFAULT_LINGAUSSIAN).unwrap();
        assert_eq!(lingaussian_bins[0], 0.00001);
        assert_eq!(lingaussian_bins[1], 0.00002);
        assert_eq!(lingaussian_bins[127], 200.0);
    }
}
