// Package mitch implements the MITCH binary protocol for financial market data.
//
// All wire formats are little-endian. Structs use manual serialization because
// Go does not support packed struct layouts via cgo-free means.
//
// Binary layout reference (all little-endian):
//
//	MitchHeader 16 bytes
//	Tick        32 bytes
//	Trade       24 bytes
//	Order       32 bytes
//	Index       40 bytes
//	Bin          8 bytes
//	OrderBook 2072 bytes
//	Bar        128 bytes
package mitch

import (
	"encoding/binary"
	"errors"
	"math"
)

// ============================================================
// Size constants
// ============================================================

const (
	SizeHeader    = 16
	SizeTick      = 32
	SizeTrade     = 24
	SizeOrder     = 32
	SizeIndex     = 40
	SizeBin       = 8
	SizeOrderBook = 2072
	SizeBar       = 128
)

// ============================================================
// Enums
// ============================================================

// MessageType represents the MITCH message type byte (ASCII).
type MessageType byte

const (
	MsgTick      MessageType = 's' // 115
	MsgTrade     MessageType = 't' // 116
	MsgOrder     MessageType = 'o' // 111
	MsgIndex     MessageType = 'i' // 105
	MsgOrderBook MessageType = 'b' //  98
	MsgBar       MessageType = 'k' // 107
)

// WireCode is the 4-bit wire code stored in TypeProvider[3:0].
type WireCode uint8

const (
	WireTrade     WireCode = 1
	WireOrder     WireCode = 2
	WireTick      WireCode = 3
	WireIndex     WireCode = 4
	WireOrderBook WireCode = 5
	WireBar       WireCode = 6
)

// WireToAscii converts a 4-bit wire code to its ASCII message type byte.
func WireToAscii(code WireCode) MessageType {
	tbl := [7]MessageType{0, MsgTrade, MsgOrder, MsgTick, MsgIndex, MsgOrderBook, MsgBar}
	if code <= 6 {
		return tbl[code]
	}
	return 0
}

// AsciiToWire converts an ASCII message type byte to its 4-bit wire code.
func AsciiToWire(ch MessageType) WireCode {
	switch ch {
	case MsgTrade:
		return WireTrade
	case MsgOrder:
		return WireOrder
	case MsgTick:
		return WireTick
	case MsgIndex:
		return WireIndex
	case MsgOrderBook:
		return WireOrderBook
	case MsgBar:
		return WireBar
	default:
		return 0
	}
}

// OrderSide represents the side of an order or trade.
type OrderSide uint8

const (
	SideBuy  OrderSide = 0
	SideSell OrderSide = 1
)

// OrderType represents the type of an order (stored in bits [7:1] of type_and_side).
type OrderType uint8

const (
	OrderMarket OrderType = 0
	OrderLimit  OrderType = 1
	OrderStop   OrderType = 2
	OrderCancel OrderType = 3
)

// AssetClass is a 4-bit asset class code used in ticker ID encoding.
type AssetClass uint8

const (
	AssetEQ  AssetClass = 0  // Equities
	AssetCB  AssetClass = 1  // Corporate bonds
	AssetSD  AssetClass = 2  // Sovereign debt
	AssetFX  AssetClass = 3  // Forex
	AssetCM  AssetClass = 4  // Commodities
	AssetRE  AssetClass = 5  // Real estate
	AssetCR  AssetClass = 6  // Crypto assets
	AssetPM  AssetClass = 7  // Private markets
	AssetCL  AssetClass = 8  // Collectibles
	AssetIN  AssetClass = 9  // Infrastructure
	AssetIP  AssetClass = 10 // Indices/products
	AssetSP  AssetClass = 11 // Structured products
	AssetCE  AssetClass = 12 // Cash equivalents
	AssetLR  AssetClass = 13 // Loans/receivables
)

// InstrType is a 4-bit instrument type code used in ticker ID encoding.
type InstrType uint8

const (
	InstrSpot   InstrType = 0
	InstrFut    InstrType = 1
	InstrFwd    InstrType = 2
	InstrSwap   InstrType = 3
	InstrPerp   InstrType = 4
	InstrCfd    InstrType = 5
	InstrCall   InstrType = 6
	InstrPut    InstrType = 7
	InstrDigi   InstrType = 8
	InstrBar    InstrType = 9
	InstrWar    InstrType = 10
	InstrPred   InstrType = 11
	InstrFund   InstrType = 12
	InstrStruct InstrType = 13
)

// BinAgg is the order book bin aggregation method.
type BinAgg uint8

const (
	BinTrilinear   BinAgg = 0
	BinLingaussian BinAgg = 1
	BinBilingeo    BinAgg = 2
	BinLingeoflat  BinAgg = 3
)

// ============================================================
// Known market provider IDs
// ============================================================

const (
	ProviderBinance   uint16 = 101
	ProviderBingX     uint16 = 111
	ProviderBitget    uint16 = 141
	ProviderBitMart   uint16 = 161
	ProviderBitstamp  uint16 = 181
	ProviderBitunix   uint16 = 191
	ProviderBullish   uint16 = 251
	ProviderBybit     uint16 = 261
	ProviderCoinbase  uint16 = 341
	ProviderCryptoCom uint16 = 391
	ProviderGate      uint16 = 561
	ProviderGroveX    uint16 = 583
	ProviderHTX       uint16 = 641
	ProviderKraken    uint16 = 721
	ProviderKuCoin    uint16 = 741
	ProviderLBank     uint16 = 761
	ProviderMEXC      uint16 = 821
	ProviderOKX       uint16 = 911
	ProviderToobit    uint16 = 1181
	ProviderUpbit     uint16 = 1251
	ProviderWhiteBIT  uint16 = 1281
	ProviderXT        uint16 = 1301
)

// ============================================================
// Errors
// ============================================================

var (
	ErrBufferTooShort = errors.New("mitch: buffer too short")
)

// ============================================================
// MitchMessage interface
// ============================================================

// MitchMessage is implemented by all MITCH body types.
type MitchMessage interface {
	// Pack serialises the message to its canonical wire bytes.
	Pack() []byte
	// MessageType returns the MITCH message type byte for this body.
	MessageType() MessageType
}

// ============================================================
// Internal u48 helpers
// ============================================================

// readU48 reads a 6-byte little-endian u48 from b and returns it as uint64.
func readU48(b []byte) uint64 {
	_ = b[5] // bounds check hint
	return uint64(b[0]) |
		uint64(b[1])<<8 |
		uint64(b[2])<<16 |
		uint64(b[3])<<24 |
		uint64(b[4])<<32 |
		uint64(b[5])<<40
}

// writeU48 writes v (must fit in 48 bits) into exactly 6 bytes at b[0:6].
func writeU48(b []byte, v uint64) {
	_ = b[5]
	b[0] = byte(v)
	b[1] = byte(v >> 8)
	b[2] = byte(v >> 16)
	b[3] = byte(v >> 24)
	b[4] = byte(v >> 32)
	b[5] = byte(v >> 40)
}

// ============================================================
// Header
// ============================================================

// Header is the 16-byte MITCH message header.
//
// Wire layout:
//
//	[0..1]   TypeProvider : u16 LE - [3:0]=wire_code, [15:4]=provider_id
//	[2..7]   Timestamp    : u48 LE - 16µs ticks since 2010-01-01T00:00:00Z
//	[8]      Count        : u8  - batch entry count (1-255)
//	[9]      Flags        : u8  - [1:0]=version(0), [7:2]=reserved
//	[10..11] Sequence     : u16 LE - per-stream gap detection
//	[12..15] reserved     : 4 bytes
type Header struct {
	TypeProvider uint16
	Timestamp    uint64 // u48 ticks (upper 16 bits unused)
	Count        uint8
	Flags        uint8
	Sequence     uint16
}

// Pack serialises the header into exactly 16 bytes.
func (h Header) Pack() []byte {
	b := make([]byte, SizeHeader)
	binary.LittleEndian.PutUint16(b[0:2], h.TypeProvider)
	writeU48(b[2:8], h.Timestamp)
	b[8] = h.Count
	b[9] = h.Flags
	binary.LittleEndian.PutUint16(b[10:12], h.Sequence)
	// b[12..15] = 0 (reserved)
	return b
}

// UnpackHeader deserialises 16 bytes into a Header.
// Returns ErrBufferTooShort if src is shorter than 16 bytes.
func UnpackHeader(src []byte) (Header, error) {
	if len(src) < SizeHeader {
		return Header{}, ErrBufferTooShort
	}
	return Header{
		TypeProvider: binary.LittleEndian.Uint16(src[0:2]),
		Timestamp:    readU48(src[2:8]),
		Count:        src[8],
		Flags:        src[9],
		Sequence:     binary.LittleEndian.Uint16(src[10:12]),
	}, nil
}

// MsgType returns the ASCII message type extracted from TypeProvider.
func (h Header) MsgType() MessageType {
	return WireToAscii(WireCode(h.TypeProvider & 0x0F))
}

// ProviderID returns the provider ID extracted from TypeProvider.
func (h Header) ProviderID() uint16 {
	return h.TypeProvider >> 4
}

// NewHeader creates a header from components.
func NewHeader(msgType MessageType, providerID uint16, timestamp uint64, count uint8) Header {
	return Header{
		TypeProvider: (uint16(AsciiToWire(msgType)) & 0x0F) | (providerID << 4),
		Timestamp:    timestamp,
		Count:        count,
	}
}

// ============================================================
// Tick
// ============================================================

// Tick is the 32-byte tick/quote snapshot body.
//
// Wire layout:
//
//	[0..7]   TickerID  : u64 LE
//	[8..15]  BidPrice  : f64 LE
//	[16..23] AskPrice  : f64 LE
//	[24..27] BidVolume : u32 LE
//	[28..31] AskVolume : u32 LE
type Tick struct {
	TickerID  uint64
	BidPrice  float64
	AskPrice  float64
	BidVolume uint32
	AskVolume uint32
}

// MessageType implements MitchMessage.
func (Tick) MessageType() MessageType { return MsgTick }

// Pack serialises the tick into exactly 32 bytes.
func (t Tick) Pack() []byte {
	b := make([]byte, SizeTick)
	binary.LittleEndian.PutUint64(b[0:8], t.TickerID)
	binary.LittleEndian.PutUint64(b[8:16], math.Float64bits(t.BidPrice))
	binary.LittleEndian.PutUint64(b[16:24], math.Float64bits(t.AskPrice))
	binary.LittleEndian.PutUint32(b[24:28], t.BidVolume)
	binary.LittleEndian.PutUint32(b[28:32], t.AskVolume)
	return b
}

// UnpackTick deserialises 32 bytes into a Tick.
// Returns ErrBufferTooShort if src is shorter than 32 bytes.
func UnpackTick(src []byte) (Tick, error) {
	if len(src) < SizeTick {
		return Tick{}, ErrBufferTooShort
	}
	return Tick{
		TickerID:  binary.LittleEndian.Uint64(src[0:8]),
		BidPrice:  math.Float64frombits(binary.LittleEndian.Uint64(src[8:16])),
		AskPrice:  math.Float64frombits(binary.LittleEndian.Uint64(src[16:24])),
		BidVolume: binary.LittleEndian.Uint32(src[24:28]),
		AskVolume: binary.LittleEndian.Uint32(src[28:32]),
	}, nil
}

// Mid returns the mid price: (BidPrice + AskPrice) / 2.
func (t Tick) Mid() float64 { return (t.BidPrice + t.AskPrice) * 0.5 }

// Spread returns the bid-ask spread: AskPrice - BidPrice.
func (t Tick) Spread() float64 { return t.AskPrice - t.BidPrice }

// ============================================================
// Trade
// ============================================================

// Trade is the 24-byte trade execution record body.
//
// Wire layout:
//
//	[0..7]   TickerID : u64 LE
//	[8..15]  Price    : f64 LE
//	[16..19] Volume   : u32 LE
//	[20..22] TradeID  : u24 LE (3 bytes, max 16_777_215)
//	[23]     Side     : u8  (0=Buy, 1=Sell)
type Trade struct {
	TickerID uint64
	Price    float64
	Volume   uint32
	TradeID  uint32 // u24 (max 16_777_215)
	Side     OrderSide
}

// MessageType implements MitchMessage.
func (Trade) MessageType() MessageType { return MsgTrade }

// Pack serialises the trade into exactly 24 bytes.
func (t Trade) Pack() []byte {
	b := make([]byte, SizeTrade)
	binary.LittleEndian.PutUint64(b[0:8], t.TickerID)
	binary.LittleEndian.PutUint64(b[8:16], math.Float64bits(t.Price))
	binary.LittleEndian.PutUint32(b[16:20], t.Volume)
	b[20] = byte(t.TradeID)
	b[21] = byte(t.TradeID >> 8)
	b[22] = byte(t.TradeID >> 16)
	b[23] = byte(t.Side)
	return b
}

// UnpackTrade deserialises 24 bytes into a Trade.
// Returns ErrBufferTooShort if src is shorter than 24 bytes.
func UnpackTrade(src []byte) (Trade, error) {
	if len(src) < SizeTrade {
		return Trade{}, ErrBufferTooShort
	}
	return Trade{
		TickerID: binary.LittleEndian.Uint64(src[0:8]),
		Price:    math.Float64frombits(binary.LittleEndian.Uint64(src[8:16])),
		Volume:   binary.LittleEndian.Uint32(src[16:20]),
		TradeID:  uint32(src[20]) | uint32(src[21])<<8 | uint32(src[22])<<16,
		Side:     OrderSide(src[23]),
	}, nil
}

// IsBuy returns true if the trade is a buy.
func (t Trade) IsBuy() bool { return t.Side == SideBuy }

// IsSell returns true if the trade is a sell.
func (t Trade) IsSell() bool { return t.Side == SideSell }

// ============================================================
// Order
// ============================================================

// Order is the 32-byte order lifecycle event body.
//
// Wire layout:
//
//	[0..7]   TickerID     : u64 LE
//	[8..11]  OrderID      : u32 LE
//	[12..19] Price        : f64 LE
//	[20..23] Quantity     : u32 LE
//	[24]     TypeAndSide  : u8  bits[7:1]=order_type, bit[0]=side
//	[25..30] Expiry       : [u8; 6] - u48 LE ms since epoch
//	[31]     padding      : u8
type Order struct {
	TickerID uint64
	OrderID  uint32
	Price    float64
	Quantity uint32
	// TypeAndSide is the raw encoded byte: bits[7:1] = OrderType, bit[0] = OrderSide.
	// Use the helper methods to get/set individual fields.
	TypeAndSide uint8
	// ExpiryMs is the order expiry in milliseconds since epoch (u48).
	ExpiryMs uint64
}

// MessageType implements MitchMessage.
func (Order) MessageType() MessageType { return MsgOrder }

// MakeTypeAndSide encodes an OrderType and OrderSide into the single byte field.
func MakeTypeAndSide(t OrderType, s OrderSide) uint8 {
	return (uint8(t) << 1) | (uint8(s) & 0x01)
}

// Side extracts the OrderSide from TypeAndSide.
func (o Order) Side() OrderSide { return OrderSide(o.TypeAndSide & 0x01) }

// Type extracts the OrderType from TypeAndSide.
func (o Order) Type() OrderType { return OrderType((o.TypeAndSide >> 1) & 0x7F) }

// IsBuy returns true if this is a buy order.
func (o Order) IsBuy() bool { return o.Side() == SideBuy }

// IsSell returns true if this is a sell order.
func (o Order) IsSell() bool { return o.Side() == SideSell }

// Pack serialises the order into exactly 32 bytes.
func (o Order) Pack() []byte {
	b := make([]byte, SizeOrder)
	binary.LittleEndian.PutUint64(b[0:8], o.TickerID)
	binary.LittleEndian.PutUint32(b[8:12], o.OrderID)
	binary.LittleEndian.PutUint64(b[12:20], math.Float64bits(o.Price))
	binary.LittleEndian.PutUint32(b[20:24], o.Quantity)
	b[24] = o.TypeAndSide
	writeU48(b[25:31], o.ExpiryMs)
	// b[31] = 0 (padding)
	return b
}

// UnpackOrder deserialises 32 bytes into an Order.
// Returns ErrBufferTooShort if src is shorter than 32 bytes.
func UnpackOrder(src []byte) (Order, error) {
	if len(src) < SizeOrder {
		return Order{}, ErrBufferTooShort
	}
	return Order{
		TickerID:    binary.LittleEndian.Uint64(src[0:8]),
		OrderID:     binary.LittleEndian.Uint32(src[8:12]),
		Price:       math.Float64frombits(binary.LittleEndian.Uint64(src[12:20])),
		Quantity:    binary.LittleEndian.Uint32(src[20:24]),
		TypeAndSide: src[24],
		ExpiryMs:    readU48(src[25:31]),
	}, nil
}

// ============================================================
// Index
// ============================================================

// Index is the 40-byte aggregated index snapshot body.
//
// Wire layout:
//
//	[0..7]   TickerID   : u64 LE
//	[8..15]  Bid        : f64 LE
//	[16..23] Ask        : f64 LE
//	[24..27] VBid       : u32 LE
//	[28..31] VAsk       : u32 LE
//	[32..33] CI         : u16 LE  (confidence interval, micro basis points)
//	[34..35] TickCount  : u16 LE
//	[36]     Confidence : u8
//	[37]     Accepted   : u8
//	[38]     Rejected   : u8
//	[39]     padding    : u8
type Index struct {
	TickerID   uint64
	Bid        float64
	Ask        float64
	VBid       uint32
	VAsk       uint32
	CI         uint16
	TickCount  uint16
	Confidence uint8
	Accepted   uint8
	Rejected   uint8
}

// MessageType implements MitchMessage.
func (Index) MessageType() MessageType { return MsgIndex }

// IsConfident returns true if Confidence >= minConfidence.
func (idx Index) IsConfident(minConfidence uint8) bool {
	return idx.Confidence >= minConfidence
}

// Spread returns the bid-ask spread: Ask - Bid.
func (idx Index) Spread() float64 { return idx.Ask - idx.Bid }

// Pack serialises the index into exactly 40 bytes (1 byte padding at end).
func (idx Index) Pack() []byte {
	b := make([]byte, SizeIndex) // zero-initialised, padding is implicitly zero
	binary.LittleEndian.PutUint64(b[0:8], idx.TickerID)
	binary.LittleEndian.PutUint64(b[8:16], math.Float64bits(idx.Bid))
	binary.LittleEndian.PutUint64(b[16:24], math.Float64bits(idx.Ask))
	binary.LittleEndian.PutUint32(b[24:28], idx.VBid)
	binary.LittleEndian.PutUint32(b[28:32], idx.VAsk)
	binary.LittleEndian.PutUint16(b[32:34], idx.CI)
	binary.LittleEndian.PutUint16(b[34:36], idx.TickCount)
	b[36] = idx.Confidence
	b[37] = idx.Accepted
	b[38] = idx.Rejected
	// b[39] = 0 (padding)
	return b
}

// UnpackIndex deserialises 40 bytes into an Index.
// Returns ErrBufferTooShort if src is shorter than 40 bytes.
func UnpackIndex(src []byte) (Index, error) {
	if len(src) < SizeIndex {
		return Index{}, ErrBufferTooShort
	}
	return Index{
		TickerID:   binary.LittleEndian.Uint64(src[0:8]),
		Bid:        math.Float64frombits(binary.LittleEndian.Uint64(src[8:16])),
		Ask:        math.Float64frombits(binary.LittleEndian.Uint64(src[16:24])),
		VBid:       binary.LittleEndian.Uint32(src[24:28]),
		VAsk:       binary.LittleEndian.Uint32(src[28:32]),
		CI:         binary.LittleEndian.Uint16(src[32:34]),
		TickCount:  binary.LittleEndian.Uint16(src[34:36]),
		Confidence: src[36],
		Accepted:   src[37],
		Rejected:   src[38],
	}, nil
}

// ============================================================
// Bin
// ============================================================

// Bin is an 8-byte order book price-level bin.
//
// Wire layout:
//
//	[0..3] OrderCount : u32 LE
//	[4..7] Volume     : u32 LE
type Bin struct {
	OrderCount uint32
	Volume     uint32
}

// Pack serialises the bin into exactly 8 bytes.
func (b Bin) Pack() []byte {
	buf := make([]byte, SizeBin)
	binary.LittleEndian.PutUint32(buf[0:4], b.OrderCount)
	binary.LittleEndian.PutUint32(buf[4:8], b.Volume)
	return buf
}

// UnpackBin deserialises 8 bytes into a Bin.
// Returns ErrBufferTooShort if src is shorter than 8 bytes.
func UnpackBin(src []byte) (Bin, error) {
	if len(src) < SizeBin {
		return Bin{}, ErrBufferTooShort
	}
	return Bin{
		OrderCount: binary.LittleEndian.Uint32(src[0:4]),
		Volume:     binary.LittleEndian.Uint32(src[4:8]),
	}, nil
}

// ============================================================
// OrderBook
// ============================================================

// OrderBook is the 2072-byte aggregated order book snapshot body.
//
// Wire layout:
//
//	[0..7]       TickerID      : u64 LE
//	[8..15]      MidPrice      : f64 LE
//	[16]         BinAggregator : u8
//	[17..23]     padding       : [u8; 7]
//	[24..1047]   Bids          : [Bin; 128]
//	[1048..2071] Asks          : [Bin; 128]
type OrderBook struct {
	TickerID      uint64
	MidPrice      float64
	BinAggregator BinAgg
	Bids          [128]Bin
	Asks          [128]Bin
}

// MessageType implements MitchMessage.
func (OrderBook) MessageType() MessageType { return MsgOrderBook }

// Pack serialises the order book into exactly 2072 bytes.
func (ob OrderBook) Pack() []byte {
	b := make([]byte, SizeOrderBook)
	binary.LittleEndian.PutUint64(b[0:8], ob.TickerID)
	binary.LittleEndian.PutUint64(b[8:16], math.Float64bits(ob.MidPrice))
	b[16] = byte(ob.BinAggregator)
	// b[17..23] = 0 (padding)
	off := 24
	for i := 0; i < 128; i++ {
		binary.LittleEndian.PutUint32(b[off:off+4], ob.Bids[i].OrderCount)
		binary.LittleEndian.PutUint32(b[off+4:off+8], ob.Bids[i].Volume)
		off += 8
	}
	for i := 0; i < 128; i++ {
		binary.LittleEndian.PutUint32(b[off:off+4], ob.Asks[i].OrderCount)
		binary.LittleEndian.PutUint32(b[off+4:off+8], ob.Asks[i].Volume)
		off += 8
	}
	return b
}

// UnpackOrderBook deserialises 2072 bytes into an OrderBook.
// Returns ErrBufferTooShort if src is shorter than 2072 bytes.
func UnpackOrderBook(src []byte) (OrderBook, error) {
	if len(src) < SizeOrderBook {
		return OrderBook{}, ErrBufferTooShort
	}
	var ob OrderBook
	ob.TickerID = binary.LittleEndian.Uint64(src[0:8])
	ob.MidPrice = math.Float64frombits(binary.LittleEndian.Uint64(src[8:16]))
	ob.BinAggregator = BinAgg(src[16])
	off := 24
	for i := 0; i < 128; i++ {
		ob.Bids[i].OrderCount = binary.LittleEndian.Uint32(src[off : off+4])
		ob.Bids[i].Volume = binary.LittleEndian.Uint32(src[off+4 : off+8])
		off += 8
	}
	for i := 0; i < 128; i++ {
		ob.Asks[i].OrderCount = binary.LittleEndian.Uint32(src[off : off+4])
		ob.Asks[i].Volume = binary.LittleEndian.Uint32(src[off+4 : off+8])
		off += 8
	}
	return ob, nil
}

// ============================================================
// Ticker ID encode / decode
//
// Bit layout (64-bit):
//
//	[63:60] InstrumentType  (4 bits)
//	[59:56] BaseAssetClass  (4 bits)
//	[55:40] BaseAssetID     (16 bits)
//	[39:36] QuoteAssetClass (4 bits)
//	[35:20] QuoteAssetID    (16 bits)
//	[19:0]  SubType         (20 bits)
// ============================================================

// TickerComponents holds the decoded components of a ticker ID.
type TickerComponents struct {
	InstrType  InstrType
	BaseClass  AssetClass
	BaseID     uint16
	QuoteClass AssetClass
	QuoteID    uint16
	SubType    uint32
}

// TickerEncode encodes a TickerComponents into a 64-bit ticker ID.
func TickerEncode(c TickerComponents) uint64 {
	return (uint64(c.InstrType&0x0F) << 60) |
		(uint64(c.BaseClass&0x0F) << 56) |
		(uint64(c.BaseID) << 40) |
		(uint64(c.QuoteClass&0x0F) << 36) |
		(uint64(c.QuoteID) << 20) |
		uint64(c.SubType&0xFFFFF)
}

// TickerDecode decodes a 64-bit ticker ID into its components.
func TickerDecode(id uint64) TickerComponents {
	return TickerComponents{
		InstrType:  InstrType((id >> 60) & 0x0F),
		BaseClass:  AssetClass((id >> 56) & 0x0F),
		BaseID:     uint16((id >> 40) & 0xFFFF),
		QuoteClass: AssetClass((id >> 36) & 0x0F),
		QuoteID:    uint16((id >> 20) & 0xFFFF),
		SubType:    uint32(id & 0xFFFFF),
	}
}

// ============================================================
// Channel ID utilities
//
// 32-bit layout: [market_provider:16][message_type:8][padding:8]
// ============================================================

// ChannelID generates a 32-bit channel ID for pub/sub routing.
func ChannelID(providerID uint16, msgType MessageType) uint32 {
	return (uint32(providerID) << 16) | (uint32(msgType) << 8)
}

// ChannelProvider extracts the market provider ID from a channel ID.
func ChannelProvider(cid uint32) uint16 {
	return uint16(cid >> 16)
}

// ChannelMsgType extracts the MessageType from a channel ID.
func ChannelMsgType(cid uint32) MessageType {
	return MessageType(cid >> 8)
}
