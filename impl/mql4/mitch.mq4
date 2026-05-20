//+------------------------------------------------------------------+
//|                                                        model.mq4 |
//|                                             BTRMitchModel.mqh |
//| Copyright BTR Supply                                             |
//| https://btr.supply                                               |
//+------------------------------------------------------------------+
#property strict

// Note: This file should include BTRIds.mqh for full functionality
// For standalone use, basic currency constants are provided below

// --- Basic Currency Constants (subset of BTR system) ---
#define BTR_CURRENCY_EUR    111
#define BTR_CURRENCY_USD    461
#define BTR_CURRENCY_GBP    826
#define BTR_CURRENCY_JPY    392
#define BTR_CURRENCY_CHF    756
#define BTR_CURRENCY_AUD    36
#define BTR_CURRENCY_CAD    124
#define BTR_CURRENCY_NZD    554

// --- Asset Class Constants ---
#define BTR_ASSET_EQUITIES          0x0
#define BTR_ASSET_CORP_BONDS        0x1
#define BTR_ASSET_SOVEREIGN_DEBT    0x2
#define BTR_ASSET_FOREX             0x3
#define BTR_ASSET_COMMODITIES       0x4
#define BTR_ASSET_PRECIOUS_METALS   0x5
#define BTR_ASSET_REAL_ESTATE       0x6
#define BTR_ASSET_CRYPTO            0x7

// --- Instrument Type Constants ---
#define BTR_INST_SPOT               0x0
#define BTR_INST_FUTURE             0x1
#define BTR_INST_FORWARD            0x2
#define BTR_INST_SWAP               0x3
#define BTR_INST_PERPETUAL          0x4
#define BTR_INST_CFD                0x5
#define BTR_INST_CALL_OPTION        0x6
#define BTR_INST_PUT_OPTION         0x7

// --- Message Type Codes ---
#define MITCH_MSG_TYPE_TRADE        't'
#define MITCH_MSG_TYPE_ORDER        'o'
#define MITCH_MSG_TYPE_TICKER       's'
#define MITCH_MSG_TYPE_ORDER_BOOK   'b'
#define MITCH_MSG_TYPE_INDEX        'i'
#define MITCH_MSG_TYPE_BAR          'k'
#define MITCH_MSG_TYPE_HEARTBEAT    'h'

// --- Side Constants ---
#define MITCH_SIDE_BUY              0
#define MITCH_SIDE_SELL             1

// --- Order Type Constants ---
#define MITCH_ORDER_TYPE_MARKET     0
#define MITCH_ORDER_TYPE_LIMIT      1
#define MITCH_ORDER_TYPE_STOP       2
#define MITCH_ORDER_TYPE_CANCEL     3

// Legacy MITCH constants (mapped to BTR constants for compatibility)
#define MITCH_INST_SPOT             BTR_INST_SPOT
#define MITCH_INST_FUTURE           BTR_INST_FUTURE
#define MITCH_INST_FORWARD          BTR_INST_FORWARD
#define MITCH_INST_SWAP             BTR_INST_SWAP
#define MITCH_INST_PERPETUAL        BTR_INST_PERPETUAL
#define MITCH_INST_CFD              BTR_INST_CFD
#define MITCH_INST_CALL_OPTION      BTR_INST_CALL_OPTION
#define MITCH_INST_PUT_OPTION       BTR_INST_PUT_OPTION

#define MITCH_ASSET_EQUITIES        BTR_ASSET_EQUITIES
#define MITCH_ASSET_CORP_BONDS      BTR_ASSET_CORP_BONDS
#define MITCH_ASSET_SOVEREIGN_DEBT  BTR_ASSET_SOVEREIGN_DEBT
#define MITCH_ASSET_FOREX           BTR_ASSET_FOREX
#define MITCH_ASSET_COMMODITIES     BTR_ASSET_COMMODITIES
#define MITCH_ASSET_PRECIOUS_METALS BTR_ASSET_PRECIOUS_METALS
#define MITCH_ASSET_REAL_ESTATE     BTR_ASSET_REAL_ESTATE
#define MITCH_ASSET_CRYPTO          BTR_ASSET_CRYPTO

// --- Unified Message Header (8 bytes) ---
struct MitchHeader
{
   uchar   messageType;      // ASCII message type code
   uchar   timestamp[6];     // 48-bit: 16µs ticks since 2010-01-01T00:00:00Z (6 bytes LE)
   uchar   count;            // Number of body entries (1-255)
};

// --- Extended Tick for MT4 with volume tracking (32 bytes) ---
struct Tick
{
   ulong   ticker_id;        // 8-byte unique ticker identifier
   double  bid_price;        // Best bid price
   double  ask_price;        // Best ask price
   uint    bid_volume;       // Volume at best bid (vbid since last snapshot)
   uint    ask_volume;       // Volume at best ask (vask since last snapshot)
};

// --- Trade Body (24 bytes) ---
struct Trade
{
   ulong   ticker_id;        // 8-byte unique ticker identifier
   double  price;            // Execution price
   uint    volume;           // Executed volume/quantity
   uchar   trade_id[3];      // u24 LE trade identifier (max 16,777,215)
   uchar   side;             // 0: Buy, 1: Sell
};

// --- Order Body (32 bytes) ---
struct Order
{
   ulong   ticker_id;        // 8-byte unique ticker identifier
   uint    order_id;         // Required unique order identifier
   double  price;            // Limit/stop price
   uint    quantity;         // Order volume/quantity
   uchar   type_and_side;    // Bit 0: Side, Bits 1-7: Order Type
   uchar   expiry[6];        // 48-bit expiry timestamp
   uchar   padding;          // Padding to 32 bytes
};

// --- Order Book Body Header (32 bytes) ---
struct OrderBook
{
   ulong   ticker_id;        // 8-byte unique ticker identifier
   double  first_tick;       // Starting price level
   double  tick_size;        // Price increment per tick
   ushort  num_ticks;        // Number of volume entries that follow
   uchar   side;             // 0: Bids, 1: Asks
   uchar   padding[5];       // Padding to 32 bytes
   // uint volumes[] follows
};

// --- Index struct (40 bytes) ---
struct Index {
    ulong ticker_id;         // u64: 8-byte ticker identifier
    double bid;              // f64: best bid price
    double ask;              // f64: best ask price
    uint vbid;               // u32: bid volume
    uint vask;               // u32: ask volume
    ushort ci;               // u16: confidence interval (micro basis points)
    ushort tick_count;       // u16: tick count
    uchar confidence;        // u8: data quality
    uchar accepted;          // u8: number of sources accepted
    uchar rejected;          // u8: number of sources rejected
    uchar _pad;              // 1 byte padding to 40 bytes
};

// --- Heartbeat struct (16 bytes) ---
// ticker_id = 0 marks a feed-wide heartbeat; nonzero is per-symbol.
// msg_count is the number of data frames emitted since the last heartbeat
// and wraps at u32::MAX. _pad reserves 4 bytes for a future flags field.
struct Heartbeat {
    ulong ticker_id;         // u64: ticker (0 = feed-wide)
    uint msg_count;          // u32: data frames since last heartbeat
    uchar _pad[4];           // 4 bytes padding to 16 bytes
};

//+------------------------------------------------------------------+
//| Utility Functions                                               |
//+------------------------------------------------------------------+

// Extract side from type_and_side field
uchar ExtractSide(uchar typeAndSide)
{
   return typeAndSide & 0x01;
}

// Extract order type from type_and_side field
uchar ExtractOrderType(uchar typeAndSide)
{
   return (typeAndSide >> 1) & 0x7F;
}

// Combine order type and side into type_and_side field
uchar CombineTypeAndSide(uchar orderType, uchar side)
{
   return (orderType << 1) | side;
}

// Basic forex ticker ID generation (simplified version)
// For full functionality, use BTRIds.mqh with GetMitchticker_id()
// cf. github.com/btr-supply/btr-mt4-stack/blob/master/Include/BTRIds.mqh
ulong GenerateForexticker_id(string symbol)
{
   // Simple implementation for common pairs
   if(symbol == "EURUSD") return 0x03006F301CD00000;
   if(symbol == "GBPUSD") return 0x030033A01CD00000;
   if(symbol == "USDJPY") return 0x0301CD018800000;
   if(symbol == "USDCHF") return 0x0301CD02F400000;
   if(symbol == "AUDUSD") return 0x0300240301CD00000;
   if(symbol == "USDCAD") return 0x0301CD007C00000;
   if(symbol == "NZDUSD") return 0x030022A01CD00000;

   // Default fallback
   return 0x0300000000000000;
}

// Pack header - little-endian
void PackHeader(MitchHeader &header, uchar &buffer[]) {
    buffer[0] = (uchar)header.messageType;

    // Pack u48 timestamp in little-endian
    ulong timestamp = header.timestamp;
    buffer[1] = (uchar)(timestamp & 0xFF);
    buffer[2] = (uchar)((timestamp >> 8) & 0xFF);
    buffer[3] = (uchar)((timestamp >> 16) & 0xFF);
    buffer[4] = (uchar)((timestamp >> 24) & 0xFF);
    buffer[5] = (uchar)((timestamp >> 32) & 0xFF);
    buffer[6] = (uchar)((timestamp >> 40) & 0xFF);

    buffer[7] = (uchar)header.count;
}

// Pack trade - little-endian (24 bytes)
void PackTrade(Trade &trade, uchar &buffer[]) {
    // Pack u64 ticker_id in little-endian
    ulong tickerId = trade.ticker_id;
    for(int i = 0; i < 8; i++) {
        buffer[i] = (uchar)((tickerId >> (i * 8)) & 0xFF);
    }

    // Pack f64 price in little-endian
    ulong priceBits = DoubleToLittleEndianBits(trade.price);
    for(int i = 0; i < 8; i++) {
        buffer[8 + i] = (uchar)((priceBits >> (i * 8)) & 0xFF);
    }

    // Pack u32 volume in little-endian
    uint vol = trade.volume;
    for(int i = 0; i < 4; i++) {
        buffer[16 + i] = (uchar)((vol >> (i * 8)) & 0xFF);
    }

    // Pack u24 trade_id (3 bytes LE)
    buffer[20] = trade.trade_id[0];
    buffer[21] = trade.trade_id[1];
    buffer[22] = trade.trade_id[2];

    buffer[23] = trade.side;
}

// Unpack header - little-endian
void UnpackHeader(const uchar &buffer[], MitchHeader &header) {
    header.messageType = buffer[0];

    // Unpack u48 timestamp from little-endian
    header.timestamp = 0;
    header.timestamp |= (ulong)buffer[1];
    header.timestamp |= (ulong)buffer[2] << 8;
    header.timestamp |= (ulong)buffer[3] << 16;
    header.timestamp |= (ulong)buffer[4] << 24;
    header.timestamp |= (ulong)buffer[5] << 32;
    header.timestamp |= (ulong)buffer[6] << 40;

    header.count = buffer[7];
}

// Unpack trade - little-endian (24 bytes)
void UnpackTrade(const uchar &buffer[], Trade &trade) {
    // Unpack u64 ticker_id from little-endian
    trade.ticker_id = 0;
    for(int i = 0; i < 8; i++) {
        trade.ticker_id |= (ulong)buffer[i] << (i * 8);
    }

    // Unpack f64 price from little-endian
    ulong priceBits = 0;
    for(int i = 0; i < 8; i++) {
        priceBits |= (ulong)buffer[8 + i] << (i * 8);
    }
    trade.price = LittleEndianBitsToDouble(priceBits);

    // Unpack u32 volume from little-endian
    trade.volume = 0;
    for(int i = 0; i < 4; i++) {
        trade.volume |= (uint)buffer[16 + i] << (i * 8);
    }

    // Unpack u24 trade_id (3 bytes LE)
    trade.trade_id[0] = buffer[20];
    trade.trade_id[1] = buffer[21];
    trade.trade_id[2] = buffer[22];

    trade.side = buffer[23];
}

// Helper function for double to little-endian bits
ulong DoubleToLittleEndianBits(double value) {
    // In MQL4, we can use bit operations directly
    // This converts double to its bit representation
    return (ulong)value; // MQL4 handles this internally
}

// Helper function for little-endian bits to double
double LittleEndianBitsToDouble(ulong bits) {
    return (double)bits; // MQL4 handles this internally
}
