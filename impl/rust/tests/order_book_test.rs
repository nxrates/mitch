use mitch::*;

#[test]
fn sizes() {
    assert_eq!(core::mem::size_of::<Bin>(), 8);
    assert_eq!(core::mem::size_of::<OrderBook>(), 2072);
}

#[test]
fn test_pack_unpack() {
    let mut bids = [Bin::default(); 128];
    bids[0] = Bin::new(5, 1000);
    let mut asks = [Bin::default(); 128];
    asks[0] = Bin::new(3, 800);

    let original = OrderBook::new(0x123456789ABCDEF0, 100.5, 0, bids, asks);
    let packed = original.pack();
    let unpacked = OrderBook::unpack(&packed).unwrap();

    let original_ticker_id = unsafe { std::ptr::addr_of!(original.ticker).read_unaligned() };
    let unpacked_ticker_id = unsafe { std::ptr::addr_of!(unpacked.ticker).read_unaligned() };
    assert_eq!(original_ticker_id, unpacked_ticker_id);

    let original_mid_price = unsafe { std::ptr::addr_of!(original.mid_price).read_unaligned() };
    let unpacked_mid_price = unsafe { std::ptr::addr_of!(unpacked.mid_price).read_unaligned() };
    assert_eq!(original_mid_price, unpacked_mid_price);

    let original_bin_aggregator = unsafe { std::ptr::addr_of!(original.bin_aggregator).read_unaligned() };
    let unpacked_bin_aggregator = unsafe { std::ptr::addr_of!(unpacked.bin_aggregator).read_unaligned() };
    assert_eq!(original_bin_aggregator, unpacked_bin_aggregator);

    let original_bids = unsafe { std::ptr::addr_of!(original.bids).read_unaligned() };
    let unpacked_bids = unsafe { std::ptr::addr_of!(unpacked.bids).read_unaligned() };
    assert_eq!(original_bids, unpacked_bids);

    let original_asks = unsafe { std::ptr::addr_of!(original.asks).read_unaligned() };
    let unpacked_asks = unsafe { std::ptr::addr_of!(unpacked.asks).read_unaligned() };
    assert_eq!(original_asks, unpacked_asks);
}

#[test]
fn volumes() {
    let mut bids = [Bin::default(); 128];
    bids[0] = Bin::new(1, 100);
    bids[1] = Bin::new(2, 200);
    let asks = [Bin::default(); 128];

    let ob = OrderBook::new(1, 100.0, 0, bids, asks);
    assert_eq!(ob.total_bid_volume(), 300);
    assert_eq!(ob.total_ask_volume(), 0);
}

#[test]
fn validation() {
    let ob = OrderBook::new(1, 100.0, 0, [Bin::default(); 128], [Bin::default(); 128]);
    assert!(ob.validate().is_ok());

    let mut invalid = ob;
    invalid.mid_price = 0.0;
    assert!(invalid.validate().is_err());

    invalid.mid_price = 100.0;
    invalid.bin_aggregator = 4;
    assert!(invalid.validate().is_err());
}

#[test]
fn batch() {
    let ob1 = OrderBook::new(1, 100.0, 0, [Bin::default(); 128], [Bin::default(); 128]);
    let ob2 = OrderBook::new(2, 200.0, 1, [Bin::default(); 128], [Bin::default(); 128]);
    let packed = pack_batch(&[ob1, ob2]);
    let unpacked: Vec<OrderBook> = unpack_batch(&packed, 2).unwrap();
    let unpacked_ticker_1 = unsafe { std::ptr::addr_of!(unpacked[0].ticker).read_unaligned() };
    assert_eq!(unpacked_ticker_1, 1);
    let unpacked_ticker_2 = unsafe { std::ptr::addr_of!(unpacked[1].ticker).read_unaligned() };
    assert_eq!(unpacked_ticker_2, 2);
}
