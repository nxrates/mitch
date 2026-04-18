use mitch::*;

// =============================================================================
// TICKER ID ENCODING/DECODING TESTS
// =============================================================================

#[test]
fn test_ticker_creation_and_extraction() {
    let ticker = TickerId::new(
        InstrumentType::SPOT,
        AssetClass::FX,
        978, // EUR
        AssetClass::FX,
        840, // USD
        0,
    ).unwrap();

    assert_eq!(ticker.instrument_type(), InstrumentType::SPOT);
    assert_eq!(ticker.base_asset_class(), AssetClass::FX);
    assert_eq!(ticker.base_asset_id(), 978);
    assert_eq!(ticker.quote_asset_class(), AssetClass::FX);
    assert_eq!(ticker.quote_asset_id(), 840);
    assert_eq!(ticker.sub_type(), 0);
}

#[test]
fn test_ticker_pack_unpack() {
    let original = TickerId::new(
        InstrumentType::PERP,
        AssetClass::CR,
        1,   // BTC
        AssetClass::CR,
        2,   // USDT
        100,
    ).unwrap();

    let packed = original.pack();
    let unpacked = TickerId::unpack(&packed).unwrap();
    assert_eq!(original, unpacked);
}

#[test]
fn test_ticker_convenience_functions() {
    let eur_usd = forex_ticker(978, 840, InstrumentType::SPOT, 0).unwrap();
    assert!(eur_usd.is_forex());
    assert!(eur_usd.is_spot());
    assert!(!eur_usd.is_crypto());

    let btc_usdt = crypto_ticker(1, 2, InstrumentType::PERP, 0).unwrap();
    assert!(btc_usdt.is_crypto());
    assert!(!btc_usdt.is_spot());

    let apple = equity_ticker(1000, 840, InstrumentType::SPOT, 0).unwrap();
    assert_eq!(apple.base_asset_class(), AssetClass::EQ);
    assert_eq!(apple.quote_asset_class(), AssetClass::FX);
}

#[test]
fn test_ticker_validation() {
    // Valid ticker
    let _ticker = TickerId::new(InstrumentType::SPOT, AssetClass::FX, 978, AssetClass::FX, 840, 0).unwrap();

    // Sub-type overflow
    let result = TickerId::new(InstrumentType::SPOT, AssetClass::FX, 978, AssetClass::FX, 840, 0x100000);
    assert!(result.is_err());
}

#[test]
fn test_ticker_batch_operations() {
    let tickers = vec![
        forex_ticker(978, 840, InstrumentType::SPOT, 0).unwrap(),
        crypto_ticker(1, 2, InstrumentType::PERP, 0).unwrap(),
        equity_ticker(1000, 840, InstrumentType::SPOT, 0).unwrap(),
    ];

    let packed = pack_ticker_batch(&tickers);
    let unpacked = unpack_ticker_batch(&packed, tickers.len()).unwrap();

    assert_eq!(tickers.len(), unpacked.len());
    for (orig, unpacked) in tickers.iter().zip(unpacked.iter()) {
        assert_eq!(*orig, *unpacked);
    }
}

#[test]
fn test_bit_manipulation_accuracy() {
    let ticker = TickerId::new(
        InstrumentType::STRUCT,
        AssetClass::LR,
        65535,
        AssetClass::IN,
        65535,
        0xFFFFF,
    ).unwrap();

    assert_eq!(ticker.instrument_type(), InstrumentType::STRUCT);
    assert_eq!(ticker.base_asset_class(), AssetClass::LR);
    assert_eq!(ticker.base_asset_id(), 65535);
    assert_eq!(ticker.quote_asset_class(), AssetClass::IN);
    assert_eq!(ticker.quote_asset_id(), 65535);
    assert_eq!(ticker.sub_type(), 0xFFFFF);
}

#[test]
fn test_spec_compliance() {
    let ticker = TickerId::new(
        InstrumentType::SPOT, AssetClass::FX, 111, AssetClass::FX, 461, 0,
    ).unwrap();

    let expected_raw = (0x0u64 << 60) | (0x3u64 << 56) | (111u64 << 40)
        | (0x3u64 << 36) | (461u64 << 20) | 0u64;
    assert_eq!(ticker.raw, expected_raw);
}

#[test]
fn test_asset_pack_unpack() {
    let packed = pack_asset(AssetClass::EQ, 831);
    let (class, class_id) = unpack_asset(packed);
    assert_eq!(class, AssetClass::EQ);
    assert_eq!(class_id, 831);

    let crypto_packed = pack_asset(AssetClass::CR, 2701);
    let (crypto_class, crypto_id) = unpack_asset(crypto_packed);
    assert_eq!(crypto_class, AssetClass::CR);
    assert_eq!(crypto_id, 2701);
}
