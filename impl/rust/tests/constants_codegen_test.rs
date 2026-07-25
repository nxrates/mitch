// tests/constants_codegen_test.rs
//
// Length-parity guard for build.rs codegen. Numbers come from the original
// hand-generated constants.rs (Phase 59.R2A baseline). If a CSV row count
// changes legitimately, update both the CSV and these constants together.

#[cfg(test)]
mod codegen_lengths {
    use mitch::constants::*;

    #[test]
    fn data_array_lengths() {
        // 205 baseline + 9 rows allocated 2026-07-25 (ids 20701..21501: PEPE,
        // SHIB, BONK, ONDO, PUMP, CVX, ETC, ZRO, LISTA) — these assets were
        // previously served under FNV fallback ids.
        assert_eq!(CRYPTO_ASSETS_DATA.len(), 214, "CRYPTO_ASSETS_DATA");
        assert_eq!(MARKET_PROVIDERS_DATA.len(), 148, "MARKET_PROVIDERS_DATA");
        assert_eq!(EQUITIES_DATA.len(), 1559, "EQUITIES_DATA");
        assert_eq!(FOREX_DATA.len(), 52, "FOREX_DATA");
        assert_eq!(COMMODITIES_DATA.len(), 58, "COMMODITIES_DATA");
        assert_eq!(INDICES_DATA.len(), 78, "INDICES_DATA");
        assert_eq!(SOVEREIGN_DEBT_DATA.len(), 183, "SOVEREIGN_DEBT_DATA");
    }

    #[test]
    fn bins_present() {
        for v in [
            BinAggregator::DEFAULT_BILINGEO,
            BinAggregator::DEFAULT_LINGAUSSIAN,
            BinAggregator::DEFAULT_LINGEOFLAT,
            BinAggregator::DEFAULT_TRILINEAR,
        ] {
            let b = BINS.get(&v).expect("bin variant present");
            assert_eq!(b.len(), 128);
        }
    }
}
