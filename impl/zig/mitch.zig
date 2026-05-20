//! mitch.zig - MITCH Protocol Zig 0.13+ Implementation
//!
//! All wire formats are little-endian. Structs are `extern` for C-ABI layout
//! compatibility. Comptime size assertions guard against layout drift.
//!
//! Binary layout reference (all little-endian):
//!   Header    16 bytes
//!   Tick      32 bytes
//!   Trade     24 bytes
//!   Order     32 bytes
//!   Index     40 bytes
//!   Bin        8 bytes
//!   Bar       96 bytes
//!   OrderBook 2072 bytes

const std = @import("std");
const mem = std.mem;

// ============================================================
// ENUMS
// ============================================================

/// MITCH message type codes (ASCII).
pub const MessageType = enum(u8) {
    /// Tick/quote snapshot - 's' (115)
    tick       = 's',
    /// Trade execution - 't' (116)
    trade      = 't',
    /// Order lifecycle - 'o' (111)
    order      = 'o',
    /// Index aggregated - 'i' (105)
    index      = 'i',
    /// Order book snapshot - 'b' (98)
    order_book = 'b',
    /// Bar/candle - 'k' (107)
    bar        = 'k',
};

/// Order side.
pub const OrderSide = enum(u8) {
    buy  = 0,
    sell = 1,
};

/// Order type (stored in bits [7:1] of type_and_side).
pub const OrderType = enum(u8) {
    market = 0,
    limit  = 1,
    stop   = 2,
    cancel = 3,
};

/// Asset class (4-bit value in ticker ID).
pub const AssetClass = enum(u8) {
    eq  = 0,   // Equities
    cb  = 1,   // Corporate bonds
    sd  = 2,   // Sovereign debt
    fx  = 3,   // Forex
    cm  = 4,   // Commodities
    re  = 5,   // Real estate
    cr  = 6,   // Crypto assets
    pm  = 7,   // Private markets
    cl  = 8,   // Collectibles
    in_ = 9,   // Infrastructure
    ip  = 10,  // Indices/products
    sp  = 11,  // Structured products
    ce  = 12,  // Cash equivalents
    lr  = 13,  // Loans/receivables
};

/// Instrument type (4-bit value, ticker ID bits [63:60]).
pub const InstrType = enum(u8) {
    spot   = 0,
    fut    = 1,
    fwd    = 2,
    swap   = 3,
    perp   = 4,
    cfd    = 5,
    call_  = 6,
    put_   = 7,
    digi   = 8,
    bar    = 9,
    war    = 10,
    pred   = 11,
    fund   = 12,
    struct_ = 13,
};

/// Order book bin aggregation method.
pub const BinAgg = enum(u8) {
    trilinear   = 0,
    lingaussian = 1,
    bilingeo    = 2,
    lingeoflat  = 3,
};

// ============================================================
// SIZE CONSTANTS
// ============================================================

pub const size_header     = 16;
pub const size_tick       = 32;
pub const size_trade      = 24;
pub const size_order      = 32;
pub const size_index      = 40;
pub const size_bin        = 8;
pub const size_bar        = 96;
pub const size_order_book = 2072;

/// Message type constant for Bar.
pub const msg_bar: u8 = 'k';

// ============================================================
// WIRE CODES (compact numeric message type for header)
// ============================================================

pub const wire_trade      : u8 = 1;
pub const wire_order      : u8 = 2;
pub const wire_tick       : u8 = 3;
pub const wire_index      : u8 = 4;
pub const wire_order_book : u8 = 5;
pub const wire_bar        : u8 = 6;

/// Map a wire code (1-6) to its ASCII message type byte.
pub fn wireToAscii(wire_code: u8) ?u8 {
    return switch (wire_code) {
        wire_trade      => 't',
        wire_order      => 'o',
        wire_tick       => 's',
        wire_index      => 'i',
        wire_order_book => 'b',
        wire_bar        => 'k',
        else => null,
    };
}

/// Map an ASCII message type byte to its wire code (1-6).
pub fn asciiToWire(ascii: u8) ?u8 {
    return switch (ascii) {
        't' => wire_trade,
        'o' => wire_order,
        's' => wire_tick,
        'i' => wire_index,
        'b' => wire_order_book,
        'k' => wire_bar,
        else => null,
    };
}

// ============================================================
// KNOWN MARKET PROVIDER IDs
// ============================================================

pub const provider_binance   : u16 = 101;
pub const provider_bingx     : u16 = 111;
pub const provider_bitget    : u16 = 141;
pub const provider_bitmart   : u16 = 161;
pub const provider_bitstamp  : u16 = 181;
pub const provider_bitunix   : u16 = 191;
pub const provider_bullish   : u16 = 251;
pub const provider_bybit     : u16 = 261;
pub const provider_coinbase  : u16 = 341;
pub const provider_cryptocom : u16 = 391;
pub const provider_gate      : u16 = 561;
pub const provider_grovex    : u16 = 583;
pub const provider_htx       : u16 = 641;
pub const provider_kraken    : u16 = 721;
pub const provider_kucoin    : u16 = 741;
pub const provider_lbank     : u16 = 761;
pub const provider_mexc      : u16 = 821;
pub const provider_okx       : u16 = 911;
pub const provider_toobit    : u16 = 1181;
pub const provider_upbit     : u16 = 1251;
pub const provider_whitebit  : u16 = 1281;
pub const provider_xt        : u16 = 1301;

// ============================================================
// INTERNAL: u48 helpers
// ============================================================

/// Read a 6-byte little-endian u48 from bytes[0..6].
inline fn read_u48(bytes: *const [6]u8) u64 {
    return @as(u64, bytes[0])
         | (@as(u64, bytes[1]) << 8)
         | (@as(u64, bytes[2]) << 16)
         | (@as(u64, bytes[3]) << 24)
         | (@as(u64, bytes[4]) << 32)
         | (@as(u64, bytes[5]) << 40);
}

/// Write v (u48 range) into bytes[0..6] as little-endian.
inline fn write_u48(bytes: *[6]u8, v: u64) void {
    bytes[0] = @truncate(v);
    bytes[1] = @truncate(v >> 8);
    bytes[2] = @truncate(v >> 16);
    bytes[3] = @truncate(v >> 24);
    bytes[4] = @truncate(v >> 32);
    bytes[5] = @truncate(v >> 40);
}

// ============================================================
// WIRE STRUCTURES
// ============================================================

/// 16-byte MITCH message header.
///
/// Wire layout:
///   [0..1]   type_provider : u16 LE - bits[3:0]=wire_code, bits[15:4]=provider_id
///   [2..7]   timestamp     : u48 LE - 16µs ticks since 2010-01-01T00:00:00Z
///   [8]      count         : u8
///   [9]      flags         : u8
///   [10..11] sequence      : u16 LE
///   [12..15] _reserved     : 4 bytes
pub const Header = extern struct {
    /// Packed type+provider: bits[3:0]=wire_code, bits[15:4]=provider_id.
    type_provider: u16 align(1),
    /// 16µs ticks since 2010-01-01T00:00:00Z (u48 stored as 6 raw bytes, LE).
    timestamp: [6]u8,
    /// Number of body entries (1-255).
    count: u8,
    /// Flags byte.
    flags: u8,
    /// Sequence number.
    sequence: u16 align(1),
    /// Reserved (4 bytes).
    _reserved: [4]u8 = [_]u8{0} ** 4,

    /// Extract the ASCII message type byte from type_provider (via wire code).
    pub fn msgType(self: Header) ?u8 {
        const wire_code: u8 = @truncate(self.type_provider & 0x0F);
        return wireToAscii(wire_code);
    }

    /// Extract the provider ID from type_provider (bits [15:4]).
    pub fn providerId(self: Header) u12 {
        return @truncate((self.type_provider >> 4) & 0x0FFF);
    }

    /// Create a Header from an ASCII message type, provider ID, and other fields.
    pub fn init(ascii_type: u8, provider_id: u12, ticks: u64, count_val: u8,
                flags_val: u8, seq: u16) Header {
        const wire_code = asciiToWire(ascii_type) orelse 0;
        var h = Header{
            .type_provider = (@as(u16, provider_id) << 4) | @as(u16, wire_code & 0x0F),
            .timestamp     = [_]u8{0} ** 6,
            .count         = count_val,
            .flags         = flags_val,
            .sequence      = seq,
        };
        h.setTimestamp(ticks);
        return h;
    }

    /// Get timestamp as 16µs ticks since 2010-01-01T00:00:00Z.
    pub fn getTimestamp(self: Header) u64 {
        return read_u48(&self.timestamp);
    }

    /// Set timestamp from 16µs ticks since 2010-01-01T00:00:00Z.
    pub fn setTimestamp(self: *Header, ticks: u64) void {
        write_u48(&self.timestamp, ticks);
    }

    /// Pack into exactly 16 bytes.
    pub fn pack(self: Header) [size_header]u8 {
        var buf: [size_header]u8 = undefined;
        @memcpy(&buf, std.mem.asBytes(&self));
        return buf;
    }

    /// Unpack exactly 16 bytes into a Header.
    pub fn unpack(bytes: [size_header]u8) Header {
        var h: Header = undefined;
        @memcpy(std.mem.asBytes(&h), &bytes);
        return h;
    }
};

comptime {
    std.debug.assert(@sizeOf(Header) == size_header);
    std.debug.assert(@offsetOf(Header, "type_provider") == 0);
    std.debug.assert(@offsetOf(Header, "timestamp")     == 2);
    std.debug.assert(@offsetOf(Header, "count")         == 8);
    std.debug.assert(@offsetOf(Header, "flags")         == 9);
    std.debug.assert(@offsetOf(Header, "sequence")      == 10);
    std.debug.assert(@offsetOf(Header, "_reserved")     == 12);
}

/// 32-byte tick/quote snapshot.
///
/// Wire layout:
///   [0..7]   ticker_id  : u64 LE
///   [8..15]  bid_price  : f64 LE
///   [16..23] ask_price  : f64 LE
///   [24..27] bid_volume : u32 LE
///   [28..31] ask_volume : u32 LE
pub const Tick = extern struct {
    ticker_id:  u64,
    bid_price:  f64,
    ask_price:  f64,
    bid_volume: u32,
    ask_volume: u32,

    /// Mid price: (bid + ask) / 2.
    pub fn mid(self: Tick) f64 {
        return (self.bid_price + self.ask_price) * 0.5;
    }

    /// Bid-ask spread: ask - bid.
    pub fn spread(self: Tick) f64 {
        return self.ask_price - self.bid_price;
    }

    /// Pack into exactly 32 bytes.
    pub fn pack(self: Tick) [size_tick]u8 {
        var buf: [size_tick]u8 = undefined;
        @memcpy(&buf, std.mem.asBytes(&self));
        return buf;
    }

    /// Unpack exactly 32 bytes into a Tick.
    pub fn unpack(bytes: [size_tick]u8) Tick {
        var t: Tick = undefined;
        @memcpy(std.mem.asBytes(&t), &bytes);
        return t;
    }
};

comptime {
    std.debug.assert(@sizeOf(Tick) == size_tick);
    std.debug.assert(@offsetOf(Tick, "ticker_id")  ==  0);
    std.debug.assert(@offsetOf(Tick, "bid_price")  ==  8);
    std.debug.assert(@offsetOf(Tick, "ask_price")  == 16);
    std.debug.assert(@offsetOf(Tick, "bid_volume") == 24);
    std.debug.assert(@offsetOf(Tick, "ask_volume") == 28);
}

/// 24-byte trade execution record.
///
/// Wire layout:
///   [0..7]   ticker_id : u64 LE
///   [8..15]  price     : f64 LE
///   [16..19] volume    : u32 LE
///   [20..22] trade_id  : u24 LE (3 bytes, max 16_777_215)
///   [23]     side      : u8  (0=Buy, 1=Sell)
pub const Trade = extern struct {
    ticker_id: u64,
    price:     f64,
    volume:    u32,
    trade_id:  [3]u8,  // u24 little-endian
    side:      u8,

    /// Returns true if this is a buy trade.
    pub fn is_buy(self: Trade) bool  { return self.side == @intFromEnum(OrderSide.buy);  }
    /// Returns true if this is a sell trade.
    pub fn is_sell(self: Trade) bool { return self.side == @intFromEnum(OrderSide.sell); }

    /// Read trade_id as u32 (u24, upper byte always 0).
    pub fn get_trade_id(self: Trade) u32 {
        return @as(u32, self.trade_id[0])
             | (@as(u32, self.trade_id[1]) << 8)
             | (@as(u32, self.trade_id[2]) << 16);
    }

    /// Write trade_id from u32 (must fit in 24 bits).
    pub fn set_trade_id(self: *Trade, id: u32) void {
        self.trade_id[0] = @truncate(id);
        self.trade_id[1] = @truncate(id >> 8);
        self.trade_id[2] = @truncate(id >> 16);
    }

    /// Pack into exactly 24 bytes.
    pub fn pack(self: Trade) [size_trade]u8 {
        var buf: [size_trade]u8 = undefined;
        @memcpy(&buf, std.mem.asBytes(&self));
        return buf;
    }

    /// Unpack exactly 24 bytes into a Trade.
    pub fn unpack(bytes: [size_trade]u8) Trade {
        var t: Trade = undefined;
        @memcpy(std.mem.asBytes(&t), &bytes);
        return t;
    }
};

comptime {
    std.debug.assert(@sizeOf(Trade) == size_trade);
    std.debug.assert(@offsetOf(Trade, "ticker_id") ==  0);
    std.debug.assert(@offsetOf(Trade, "price")     ==  8);
    std.debug.assert(@offsetOf(Trade, "volume")    == 16);
    std.debug.assert(@offsetOf(Trade, "trade_id")  == 20);
    std.debug.assert(@offsetOf(Trade, "side")      == 23);
}

/// 32-byte order lifecycle event.
///
/// Wire layout:
///   [0..7]   ticker_id     : u64 LE
///   [8..11]  order_id      : u32 LE
///   [12..19] price         : f64 LE
///   [20..23] quantity      : u32 LE
///   [24]     type_and_side : u8  bits[7:1]=order_type, bit[0]=side
///   [25..30] expiry        : [u8; 6] - u48 LE ms since epoch
///   [31]     padding       : u8
pub const Order = extern struct {
    ticker_id:     u64,
    order_id:      u32,
    price:         f64,
    quantity:      u32,
    /// bits[7:1] = OrderType, bit[0] = OrderSide.
    type_and_side: u8,
    /// Expiry in milliseconds since epoch (u48, stored as 6 raw bytes LE).
    expiry:        [6]u8,
    _pad:          u8 = 0,

    /// Build type_and_side byte from explicit type and side.
    pub fn make_type_and_side(t: OrderType, s: OrderSide) u8 {
        return (@as(u8, @intFromEnum(t)) << 1) | (@as(u8, @intFromEnum(s)) & 0x01);
    }

    /// Extract OrderSide from type_and_side.
    pub fn side(self: Order) OrderSide {
        return @enumFromInt(self.type_and_side & 0x01);
    }

    /// Extract OrderType from type_and_side.
    pub fn order_type(self: Order) OrderType {
        return @enumFromInt((self.type_and_side >> 1) & 0x7F);
    }

    /// Returns true if this is a buy order.
    pub fn is_buy(self: Order) bool  { return self.side() == .buy;  }
    /// Returns true if this is a sell order.
    pub fn is_sell(self: Order) bool { return self.side() == .sell; }

    /// Get expiry as milliseconds since epoch.
    pub fn expiry_ms(self: Order) u64 {
        return read_u48(&self.expiry);
    }

    /// Set expiry from milliseconds since epoch.
    pub fn set_expiry(self: *Order, ms: u64) void {
        write_u48(&self.expiry, ms);
    }

    /// Pack into exactly 32 bytes.
    pub fn pack(self: Order) [size_order]u8 {
        var buf: [size_order]u8 = undefined;
        @memcpy(&buf, std.mem.asBytes(&self));
        return buf;
    }

    /// Unpack exactly 32 bytes into an Order.
    pub fn unpack(bytes: [size_order]u8) Order {
        var o: Order = undefined;
        @memcpy(std.mem.asBytes(&o), &bytes);
        return o;
    }
};

comptime {
    std.debug.assert(@sizeOf(Order) == size_order);
    std.debug.assert(@offsetOf(Order, "ticker_id")     ==  0);
    std.debug.assert(@offsetOf(Order, "order_id")      ==  8);
    std.debug.assert(@offsetOf(Order, "price")         == 12);
    std.debug.assert(@offsetOf(Order, "quantity")      == 20);
    std.debug.assert(@offsetOf(Order, "type_and_side") == 24);
    std.debug.assert(@offsetOf(Order, "expiry")        == 25);
}

/// 40-byte aggregated index snapshot.
///
/// Wire layout:
///   [0..7]   ticker_id  : u64 LE
///   [8..15]  bid        : f64 LE
///   [16..23] ask        : f64 LE
///   [24..27] vbid       : u32 LE
///   [28..31] vask       : u32 LE
///   [32..33] ci         : u16 LE  (confidence interval, micro basis points)
///   [34..35] tick_count : u16 LE
///   [36]     confidence : u8
///   [37]     accepted   : u8
///   [38]     rejected   : u8
///   [39]     padding    : u8
pub const Index = extern struct {
    ticker_id:  u64,
    bid:        f64,
    ask:        f64,
    vbid:       u32,
    vask:       u32,
    ci:         u16,
    tick_count: u16,
    confidence: u8,
    accepted:   u8,
    rejected:   u8,
    _pad:       u8 = 0,

    /// Returns true if confidence >= min_confidence.
    pub fn is_confident(self: Index, min_confidence: u8) bool {
        return self.confidence >= min_confidence;
    }

    /// Spread: ask - bid.
    pub fn spread(self: Index) f64 {
        return self.ask - self.bid;
    }

    /// Pack into exactly 40 bytes.
    pub fn pack(self: Index) [size_index]u8 {
        var buf: [size_index]u8 = undefined;
        @memcpy(&buf, std.mem.asBytes(&self));
        return buf;
    }

    /// Unpack exactly 40 bytes into an Index.
    pub fn unpack(bytes: [size_index]u8) Index {
        var i: Index = undefined;
        @memcpy(std.mem.asBytes(&i), &bytes);
        return i;
    }
};

comptime {
    std.debug.assert(@sizeOf(Index) == size_index);
    std.debug.assert(@offsetOf(Index, "ticker_id")  ==  0);
    std.debug.assert(@offsetOf(Index, "bid")        ==  8);
    std.debug.assert(@offsetOf(Index, "ask")        == 16);
    std.debug.assert(@offsetOf(Index, "vbid")       == 24);
    std.debug.assert(@offsetOf(Index, "vask")       == 28);
    std.debug.assert(@offsetOf(Index, "ci")         == 32);
    std.debug.assert(@offsetOf(Index, "tick_count") == 34);
    std.debug.assert(@offsetOf(Index, "confidence") == 36);
    std.debug.assert(@offsetOf(Index, "accepted")   == 37);
    std.debug.assert(@offsetOf(Index, "rejected")   == 38);
}

/// 8-byte order book price-level bin.
///
/// Wire layout:
///   [0..3] order_count : u32 LE
///   [4..7] volume      : u32 LE
pub const Bin = extern struct {
    order_count: u32,
    volume:      u32,

    /// Pack into exactly 8 bytes.
    pub fn pack(self: Bin) [size_bin]u8 {
        var buf: [size_bin]u8 = undefined;
        @memcpy(&buf, std.mem.asBytes(&self));
        return buf;
    }

    /// Unpack exactly 8 bytes into a Bin.
    pub fn unpack(bytes: [size_bin]u8) Bin {
        var b: Bin = undefined;
        @memcpy(std.mem.asBytes(&b), &bytes);
        return b;
    }
};

comptime {
    std.debug.assert(@sizeOf(Bin) == size_bin);
    std.debug.assert(@offsetOf(Bin, "order_count") == 0);
    std.debug.assert(@offsetOf(Bin, "volume")      == 4);
}

/// 2072-byte aggregated order book snapshot.
///
/// Wire layout:
///   [0..7]       ticker_id      : u64 LE
///   [8..15]      mid_price      : f64 LE
///   [16]         bin_aggregator : u8
///   [17..23]     padding        : [u8; 7]
///   [24..1047]   bids           : [Bin; 128]
///   [1048..2071] asks           : [Bin; 128]
pub const OrderBook = extern struct {
    ticker_id:      u64,
    mid_price:      f64,
    bin_aggregator: u8,
    _pad:           [7]u8 = [_]u8{0} ** 7,
    bids:           [128]Bin,
    asks:           [128]Bin,

    /// Pack into exactly 2072 bytes.
    pub fn pack(self: OrderBook) [size_order_book]u8 {
        var buf: [size_order_book]u8 = undefined;
        @memcpy(&buf, std.mem.asBytes(&self));
        return buf;
    }

    /// Unpack exactly 2072 bytes into an OrderBook.
    pub fn unpack(bytes: [size_order_book]u8) OrderBook {
        var ob: OrderBook = undefined;
        @memcpy(std.mem.asBytes(&ob), &bytes);
        return ob;
    }
};

comptime {
    std.debug.assert(@sizeOf(OrderBook) == size_order_book);
    std.debug.assert(@offsetOf(OrderBook, "ticker_id")      ==    0);
    std.debug.assert(@offsetOf(OrderBook, "mid_price")      ==    8);
    std.debug.assert(@offsetOf(OrderBook, "bin_aggregator") ==   16);
    std.debug.assert(@offsetOf(OrderBook, "bids")           ==   24);
    std.debug.assert(@offsetOf(OrderBook, "asks")           == 1048);
}

// ============================================================
// TAGGED UNION covering all message body types
// ============================================================

/// Tagged union of all MITCH body types.
pub const MitchMessage = union(MessageType) {
    tick:       Tick,
    trade:      Trade,
    order:      Order,
    index:      Index,
    order_book: OrderBook,
    bar:        void, // placeholder - Bar body type TBD
};

// ============================================================
// TICKER ID ENCODE / DECODE
//
// Bit layout (64-bit):
//   [63:60] InstrumentType  (4 bits)
//   [59:56] BaseAssetClass  (4 bits)
//   [55:40] BaseAssetID     (16 bits)
//   [39:36] QuoteAssetClass (4 bits)
//   [35:20] QuoteAssetID    (16 bits)
//   [19:0]  SubType         (20 bits)
// ============================================================

/// Decoded components of a ticker ID.
pub const TickerComponents = struct {
    instr_type:  InstrType,
    base_class:  AssetClass,
    base_id:     u16,
    quote_class: AssetClass,
    quote_id:    u16,
    sub_type:    u20,
};

/// Encode a 64-bit ticker ID from its components.
///
/// Example: BTC/USDT SPOT
///   ticker_encode(.{ .instr_type=.spot, .base_class=.cr, .base_id=2701,
///                    .quote_class=.cr, .quote_id=17601, .sub_type=0 })
pub fn ticker_encode(c: TickerComponents) u64 {
    return (@as(u64, @intFromEnum(c.instr_type))  << 60)
         | (@as(u64, @intFromEnum(c.base_class))  << 56)
         | (@as(u64, c.base_id)                   << 40)
         | (@as(u64, @intFromEnum(c.quote_class)) << 36)
         | (@as(u64, c.quote_id)                  << 20)
         | @as(u64, c.sub_type);
}

/// Decode a 64-bit ticker ID into its components.
pub fn ticker_decode(id: u64) TickerComponents {
    return .{
        .instr_type  = @enumFromInt((id >> 60) & 0x0F),
        .base_class  = @enumFromInt((id >> 56) & 0x0F),
        .base_id     = @truncate((id >> 40) & 0xFFFF),
        .quote_class = @enumFromInt((id >> 36) & 0x0F),
        .quote_id    = @truncate((id >> 20) & 0xFFFF),
        .sub_type    = @truncate(id & 0xFFFFF),
    };
}

// ============================================================
// CHANNEL ID UTILITIES
//
// 32-bit layout: [market_provider:16][message_type:8][padding:8]
// ============================================================

/// Generate a 32-bit channel ID for pub/sub routing.
pub fn channel_id(provider_id: u16, msg_type: MessageType) u32 {
    return (@as(u32, provider_id) << 16)
         | (@as(u32, @intFromEnum(msg_type)) << 8);
}

/// Extract market provider ID from a channel ID.
pub fn channel_provider(cid: u32) u16 {
    return @truncate(cid >> 16);
}

/// Extract MessageType from a channel ID.
pub fn channel_msg_type(cid: u32) MessageType {
    return @enumFromInt(@as(u8, @truncate(cid >> 8)));
}

// ============================================================
// TESTS
// ============================================================

test "sizes" {
    try std.testing.expectEqual(@as(usize, size_header),     @sizeOf(Header));
    try std.testing.expectEqual(@as(usize, size_tick),       @sizeOf(Tick));
    try std.testing.expectEqual(@as(usize, size_trade),      @sizeOf(Trade));
    try std.testing.expectEqual(@as(usize, size_order),      @sizeOf(Order));
    try std.testing.expectEqual(@as(usize, size_index),      @sizeOf(Index));
    try std.testing.expectEqual(@as(usize, size_bin),        @sizeOf(Bin));
    try std.testing.expectEqual(@as(usize, size_order_book), @sizeOf(OrderBook));
}

test "tick round-trip" {
    const original = Tick{
        .ticker_id  = 0x0060A8D644C10000,
        .bid_price  = 65432.10,
        .ask_price  = 65432.50,
        .bid_volume = 100,
        .ask_volume = 200,
    };
    const bytes = original.pack();
    const decoded = Tick.unpack(bytes);
    try std.testing.expectEqual(original.ticker_id,  decoded.ticker_id);
    try std.testing.expectEqual(original.bid_price,  decoded.bid_price);
    try std.testing.expectEqual(original.ask_price,  decoded.ask_price);
    try std.testing.expectEqual(original.bid_volume, decoded.bid_volume);
    try std.testing.expectEqual(original.ask_volume, decoded.ask_volume);
}

test "trade round-trip" {
    var original = Trade{
        .ticker_id = 0x0060A8D644C10000,
        .price     = 65432.10,
        .volume    = 5,
        .trade_id  = [_]u8{0} ** 3,
        .side      = @intFromEnum(OrderSide.buy),
    };
    original.set_trade_id(99999);
    const bytes = original.pack();
    const decoded = Trade.unpack(bytes);
    try std.testing.expectEqual(original.ticker_id,      decoded.ticker_id);
    try std.testing.expectEqual(original.price,          decoded.price);
    try std.testing.expectEqual(original.volume,         decoded.volume);
    try std.testing.expectEqual(original.get_trade_id(), decoded.get_trade_id());
    try std.testing.expectEqual(original.side,           decoded.side);
}

test "order round-trip" {
    var original = Order{
        .ticker_id     = 0x0060A8D644C10000,
        .order_id      = 42,
        .price         = 65000.0,
        .quantity      = 10,
        .type_and_side = Order.make_type_and_side(.limit, .sell),
        .expiry        = [_]u8{0} ** 6,
    };
    original.set_expiry(1_700_000_000_000);
    const bytes = original.pack();
    const decoded = Order.unpack(bytes);
    try std.testing.expectEqual(original.ticker_id,     decoded.ticker_id);
    try std.testing.expectEqual(original.order_id,      decoded.order_id);
    try std.testing.expectEqual(original.price,         decoded.price);
    try std.testing.expectEqual(original.quantity,      decoded.quantity);
    try std.testing.expectEqual(original.type_and_side, decoded.type_and_side);
    try std.testing.expectEqual(original.expiry_ms(),   decoded.expiry_ms());
}

test "ticker encode/decode round-trip" {
    const c = TickerComponents{
        .instr_type  = .spot,
        .base_class  = .cr,
        .base_id     = 2701,
        .quote_class = .cr,
        .quote_id    = 17601,
        .sub_type    = 0,
    };
    const id = ticker_encode(c);
    const d  = ticker_decode(id);
    try std.testing.expectEqual(c.instr_type,  d.instr_type);
    try std.testing.expectEqual(c.base_class,  d.base_class);
    try std.testing.expectEqual(c.base_id,     d.base_id);
    try std.testing.expectEqual(c.quote_class, d.quote_class);
    try std.testing.expectEqual(c.quote_id,    d.quote_id);
    try std.testing.expectEqual(c.sub_type,    d.sub_type);
}

test "channel id round-trip" {
    const cid = channel_id(provider_binance, .tick);
    try std.testing.expectEqual(provider_binance, channel_provider(cid));
    try std.testing.expectEqual(MessageType.tick, channel_msg_type(cid));
}

test "header timestamp round-trip" {
    var h = Header.init('s', 101, 0, 1, 0, 0);
    const ticks: u64 = 12_345_678_901_234;
    h.setTimestamp(ticks);
    try std.testing.expectEqual(ticks, h.getTimestamp());
    try std.testing.expectEqual(@as(?u8, 's'), h.msgType());
    try std.testing.expectEqual(@as(u12, 101), h.providerId());
    const bytes = h.pack();
    const decoded = Header.unpack(bytes);
    try std.testing.expectEqual(ticks, decoded.getTimestamp());
    try std.testing.expectEqual(@as(?u8, 's'), decoded.msgType());
    try std.testing.expectEqual(@as(u12, 101), decoded.providerId());
}
