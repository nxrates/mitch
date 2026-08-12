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
        // 52 baseline + 4 rows (ids 05201..05501: KES, UGX, ZMW, BWP) for the
        // cTrader African exotics.
        assert_eq!(FOREX_DATA.len(), 56, "FOREX_DATA");
        assert_eq!(COMMODITIES_DATA.len(), 58, "COMMODITIES_DATA");
        // 78 baseline + 6 rows (ids 07701..08201: MDAX, OBX, Hang Seng Tech,
        // Gold Index, TecDAX, Hang Seng China Enterprises) for the cTrader index
        // universe. The last one exists because CHINAH and its contract code HHI
        // were aliased onto China A50 and Hang Seng respectively: three distinct
        // Chinese indices sharing two ids, so a CHINAH quote marked the wrong one.
        assert_eq!(INDICES_DATA.len(), 85, "INDICES_DATA");
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
