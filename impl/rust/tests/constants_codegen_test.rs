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
        // + 5 rows allocated 2026-08-13 (ids 21601..22001: QCAD, AUDF, BRLA,
        // JPYC, KRW1) — the FX-wrapper stablecoins of the BTR FX Core pool,
        // each pegged 1:1 to a forex.csv currency.
        // + 11 rows that were added WITHOUT updating this guard, so it was
        // already red at 230 when found on 2026-08-15.
        // - 1 row: 23001 Sanctum Infinity (INF) removed the same day. It had no
        //   CEX market on any venue we scrape, and its symbol collided with two
        //   equities (Informa, New Informa), so it could only ever resolve by
        //   class disambiguation to a price we could not source.
        assert_eq!(CRYPTO_ASSETS_DATA.len(), 229, "CRYPTO_ASSETS_DATA");
        assert_eq!(MARKET_PROVIDERS_DATA.len(), 148, "MARKET_PROVIDERS_DATA");
        // 1559 -> 1667: 108 rows were added without updating this guard, so it
        // was already red when found on 2026-08-15. Recorded rather than
        // re-baselined silently: the guard only earns its keep if the number is
        // the real one.
        assert_eq!(EQUITIES_DATA.len(), 1667, "EQUITIES_DATA");
        // 52 baseline + 4 rows (ids 05201..05501: KES, UGX, ZMW, BWP) for the
        // cTrader African exotics.
        assert_eq!(FOREX_DATA.len(), 56, "FOREX_DATA");
        assert_eq!(COMMODITIES_DATA.len(), 58, "COMMODITIES_DATA");
        // 78 baseline + 7 rows (ids 07701..08301: MDAX, OBX, Hang Seng Tech,
        // Gold Index, TecDAX, Hang Seng China Enterprises, MSCI Singapore) for the
        // cTrader index universe. The last two exist because a broker symbol was
        // pointing at an index it is not: CHINAH and its contract code HHI sat on
        // China A50 and Hang Seng, and SCI25 (25 constituents) would otherwise
        // have landed on the 30-constituent Straits Times.
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
