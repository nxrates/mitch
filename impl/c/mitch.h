/**
 * mitch.h - MITCH Protocol C99 Implementation
 *
 * Single-header, C99, no external dependencies.
 * All structures use little-endian byte order.
 * All pack/unpack operations use memcpy for safe unaligned access.
 *
 * Binary layout reference (all little-endian):
 *   MitchHeader 16 bytes
 *   Tick        32 bytes
 *   Trade       24 bytes
 *   Order       32 bytes
 *   Index       40 bytes
 *   Bin          8 bytes
 *   OrderBook 2072 bytes
 *   Bar        128 bytes
 */

#pragma once

#include <assert.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

/* ============================================================
 * Compile-time platform check
 * ============================================================ */
#if defined(__BYTE_ORDER__) && __BYTE_ORDER__ != __ORDER_LITTLE_ENDIAN__
#  error "mitch.h requires a little-endian host"
#endif

/* ============================================================
 * Packed struct attribute
 * ============================================================ */
#if defined(_MSC_VER)
#  define MITCH_PACK_BEGIN __pragma(pack(push, 1))
#  define MITCH_PACK_END   __pragma(pack(pop))
#  define MITCH_PACKED
#else
#  define MITCH_PACK_BEGIN
#  define MITCH_PACK_END
#  define MITCH_PACKED __attribute__((packed))
#endif

/* ============================================================
 * Static assert helper (C99 compatible)
 * ============================================================ */
#define MITCH_STATIC_ASSERT(cond, msg) \
    typedef char mitch_static_assert_##msg[(cond) ? 1 : -1]

/* ============================================================
 * ENUMS
 * ============================================================ */

/** MITCH message type codes (ASCII). */
typedef enum MitchMessageType {
    MITCH_MSG_TICK       = 's', /* 115 - Tick/quote snapshot */
    MITCH_MSG_TRADE      = 't', /* 116 - Trade execution     */
    MITCH_MSG_ORDER      = 'o', /* 111 - Order lifecycle     */
    MITCH_MSG_INDEX      = 'i', /* 105 - Index aggregated    */
    MITCH_MSG_ORDER_BOOK = 'b', /*  98 - Order book snapshot */
    MITCH_MSG_BAR        = 'k', /* 107 - Bar/candlestick */
} MitchMessageType;

/** MITCH wire codes (compact numeric encoding for header). */
typedef enum MitchWireCode {
    MITCH_WIRE_TRADE      = 1,
    MITCH_WIRE_ORDER      = 2,
    MITCH_WIRE_TICK       = 3,
    MITCH_WIRE_INDEX      = 4,
    MITCH_WIRE_ORDER_BOOK = 5,
    MITCH_WIRE_BAR        = 6,
} MitchWireCode;

/** Order side. */
typedef enum MitchOrderSide {
    MITCH_SIDE_BUY  = 0,
    MITCH_SIDE_SELL = 1,
} MitchOrderSide;

/** Order type (stored in bits [7:1] of type_and_side). */
typedef enum MitchOrderType {
    MITCH_ORDER_MARKET = 0,
    MITCH_ORDER_LIMIT  = 1,
    MITCH_ORDER_STOP   = 2,
    MITCH_ORDER_CANCEL = 3,
} MitchOrderType;

/** Asset class (4-bit value in ticker ID). */
typedef enum MitchAssetClass {
    MITCH_AC_EQ  = 0,  /* Equities           */
    MITCH_AC_CB  = 1,  /* Corporate bonds    */
    MITCH_AC_SD  = 2,  /* Sovereign debt     */
    MITCH_AC_FX  = 3,  /* Forex              */
    MITCH_AC_CM  = 4,  /* Commodities        */
    MITCH_AC_RE  = 5,  /* Real estate        */
    MITCH_AC_CR  = 6,  /* Crypto assets      */
    MITCH_AC_PM  = 7,  /* Private markets    */
    MITCH_AC_CL  = 8,  /* Collectibles       */
    MITCH_AC_IN  = 9,  /* Infrastructure     */
    MITCH_AC_IP  = 10, /* Indices/products   */
    MITCH_AC_SP  = 11, /* Structured products*/
    MITCH_AC_CE  = 12, /* Cash equivalents   */
    MITCH_AC_LR  = 13, /* Loans/receivables  */
} MitchAssetClass;

/** Instrument type (4-bit value in ticker ID bits [63:60]). */
typedef enum MitchInstrType {
    MITCH_INSTR_SPOT   = 0,
    MITCH_INSTR_FUT    = 1,
    MITCH_INSTR_FWD    = 2,
    MITCH_INSTR_SWAP   = 3,
    MITCH_INSTR_PERP   = 4,
    MITCH_INSTR_CFD    = 5,
    MITCH_INSTR_CALL   = 6,
    MITCH_INSTR_PUT    = 7,
    MITCH_INSTR_DIGI   = 8,
    MITCH_INSTR_BAR    = 9,
    MITCH_INSTR_WAR    = 10,
    MITCH_INSTR_PRED   = 11,
    MITCH_INSTR_FUND   = 12,
    MITCH_INSTR_STRUCT = 13,
} MitchInstrType;

/** Order book bin aggregation method. */
typedef enum MitchBinAgg {
    MITCH_BIN_TRILINEAR  = 0,
    MITCH_BIN_LINGAUSSIAN = 1,
    MITCH_BIN_BILINGEO   = 2,
    MITCH_BIN_LINGEOFLAT = 3,
} MitchBinAgg;

static inline uint8_t mitch_wire_to_ascii(uint8_t code) {
    static const uint8_t tbl[] = {0, 't', 'o', 's', 'i', 'b', 'k'};
    return code <= 6 ? tbl[code] : 0;
}

static inline uint8_t mitch_ascii_to_wire(uint8_t ch) {
    switch (ch) {
        case 't': return 1; case 'o': return 2; case 's': return 3;
        case 'i': return 4; case 'b': return 5; case 'k': return 6;
        default:  return 0;
    }
}

/* ============================================================
 * WIRE STRUCTURES  (packed, do not access fields directly for
 * portability - use the pack/unpack helpers below)
 * ============================================================ */

MITCH_PACK_BEGIN

/**
 * MitchHeader - 16-byte message header.
 *
 * [0..1]   type_provider : u16 LE - [3:0]=wire_code, [15:4]=provider_id
 * [2..7]   timestamp     : u48 LE - 16µs ticks since 2010-01-01T00:00:00Z
 * [8]      count         : u8  - batch entry count (1-255)
 * [9]      flags         : u8  - [1:0]=version(0), [7:2]=reserved
 * [10..11] sequence      : u16 LE - per-stream gap detection
 * [12..15] _reserved     : 4 bytes
 */
typedef struct MITCH_PACKED MitchHeader {
    uint16_t type_provider;
    uint8_t  timestamp[6];
    uint8_t  count;
    uint8_t  flags;
    uint16_t sequence;
    uint8_t  _reserved[4];
} MitchHeader;

/**
 * MitchTick - 32-byte tick/quote snapshot.
 *
 * [0..7]   ticker_id  : u64 LE
 * [8..15]  bid_price  : f64 LE
 * [16..23] ask_price  : f64 LE
 * [24..27] bid_volume : u32 LE
 * [28..31] ask_volume : u32 LE
 */
typedef struct MITCH_PACKED MitchTick {
    uint64_t ticker_id;
    double   bid_price;
    double   ask_price;
    uint32_t bid_volume;
    uint32_t ask_volume;
} MitchTick;

/**
 * MitchTrade - 24-byte trade execution record.
 *
 * [0..7]   ticker_id : u64 LE
 * [8..15]  price     : f64 LE
 * [16..19] volume    : u32 LE
 * [20..22] trade_id  : u24 LE (3 bytes, max 16_777_215)
 * [23]     side      : u8  (0=Buy, 1=Sell)
 */
typedef struct MITCH_PACKED MitchTrade {
    uint64_t ticker_id;
    double   price;
    uint32_t volume;
    uint8_t  trade_id[3]; /* u24 little-endian */
    uint8_t  side;
} MitchTrade;

/**
 * MitchOrder - 32-byte order lifecycle event.
 *
 * [0..7]   ticker_id    : u64 LE
 * [8..11]  order_id     : u32 LE
 * [12..19] price        : f64 LE
 * [20..23] quantity     : u32 LE
 * [24]     type_and_side: u8  bits[7:1]=order_type, bit[0]=side
 * [25..30] expiry       : [u8; 6] - u48 LE milliseconds since epoch
 * [31]     padding      : u8
 */
typedef struct MITCH_PACKED MitchOrder {
    uint64_t ticker_id;
    uint32_t order_id;
    double   price;
    uint32_t quantity;
    uint8_t  type_and_side;
    uint8_t  expiry[6]; /* u48 little-endian */
    uint8_t  _pad;
} MitchOrder;

/**
 * MitchIndex - 40-byte aggregated index snapshot.
 *
 * [0..7]   ticker_id  : u64 LE
 * [8..15]  bid        : f64 LE
 * [16..23] ask        : f64 LE
 * [24..27] vbid       : u32 LE
 * [28..31] vask       : u32 LE
 * [32..33] ci         : u16 LE  (confidence interval, micro basis points)
 * [34..35] tick_count : u16 LE
 * [36]     confidence : u8
 * [37]     accepted   : u8
 * [38]     rejected   : u8
 * [39]     padding    : u8
 */
typedef struct MITCH_PACKED MitchIndex {
    uint64_t ticker_id;
    double   bid;
    double   ask;
    uint32_t vbid;
    uint32_t vask;
    uint16_t ci;
    uint16_t tick_count;
    uint8_t  confidence;
    uint8_t  accepted;
    uint8_t  rejected;
    uint8_t  _pad;
} MitchIndex;

/**
 * MitchBin - 8-byte order book price bin.
 *
 * [0..3] order_count : u32 LE
 * [4..7] volume      : u32 LE
 */
typedef struct MITCH_PACKED MitchBin {
    uint32_t order_count;
    uint32_t volume;
} MitchBin;

/**
 * MitchOrderBook - 2072-byte aggregated order book snapshot.
 *
 * [0..7]      ticker_id      : u64 LE
 * [8..15]     mid_price      : f64 LE
 * [16]        bin_aggregator : u8
 * [17..23]    padding        : [u8; 7]
 * [24..1047]  bids           : [MitchBin; 128]
 * [1048..2071] asks          : [MitchBin; 128]
 */
typedef struct MITCH_PACKED MitchOrderBook {
    uint64_t ticker_id;
    double   mid_price;
    uint8_t  bin_aggregator;
    uint8_t  _pad[7];
    MitchBin bids[128];
    MitchBin asks[128];
} MitchOrderBook;

MITCH_PACK_END

/* ============================================================
 * Compile-time size assertions
 * ============================================================ */
MITCH_STATIC_ASSERT(sizeof(MitchHeader)    ==   16, MitchHeader_size);
MITCH_STATIC_ASSERT(sizeof(MitchTick)      ==   32, MitchTick_size);
MITCH_STATIC_ASSERT(sizeof(MitchTrade)     ==   24, MitchTrade_size);
MITCH_STATIC_ASSERT(sizeof(MitchOrder)     ==   32, MitchOrder_size);
MITCH_STATIC_ASSERT(sizeof(MitchIndex)     ==   40, MitchIndex_size);
MITCH_STATIC_ASSERT(sizeof(MitchBin)       ==    8, MitchBin_size);
MITCH_STATIC_ASSERT(sizeof(MitchOrderBook) == 2072, MitchOrderBook_size);

/* ============================================================
 * TICKER ID ENCODE / DECODE
 *
 * Bit layout (64-bit, little-endian wire value):
 *   [63:60] InstrumentType  (4 bits)
 *   [59:56] BaseAssetClass  (4 bits)
 *   [55:40] BaseAssetID     (16 bits)
 *   [39:36] QuoteAssetClass (4 bits)
 *   [35:20] QuoteAssetID    (16 bits)
 *   [19:0]  SubType         (20 bits)
 * ============================================================ */

/**
 * Encode a ticker ID from its components.
 *
 * @param instr_type   Instrument type  (0-15)
 * @param base_class   Base asset class (0-15)
 * @param base_id      Base asset ID    (0-65535)
 * @param quote_class  Quote asset class(0-15)
 * @param quote_id     Quote asset ID   (0-65535)
 * @param sub_type     Sub-type         (0-1048575)
 * @return 64-bit ticker ID
 */
static inline uint64_t mitch_ticker_encode(
    MitchInstrType  instr_type,
    MitchAssetClass base_class,
    uint16_t        base_id,
    MitchAssetClass quote_class,
    uint16_t        quote_id,
    uint32_t        sub_type)
{
    return ((uint64_t)(instr_type  & 0x0F) << 60)
         | ((uint64_t)(base_class  & 0x0F) << 56)
         | ((uint64_t)(base_id            ) << 40)
         | ((uint64_t)(quote_class & 0x0F) << 36)
         | ((uint64_t)(quote_id           ) << 20)
         | ((uint64_t)(sub_type   & 0xFFFFF));
}

/** Decoded components of a ticker ID. */
typedef struct MitchTickerComponents {
    MitchInstrType  instr_type;
    MitchAssetClass base_class;
    uint16_t        base_id;
    MitchAssetClass quote_class;
    uint16_t        quote_id;
    uint32_t        sub_type;
} MitchTickerComponents;

/**
 * Decode a 64-bit ticker ID into its components.
 *
 * @param ticker_id  Raw 64-bit ticker ID
 * @param out        Pointer to result struct (must not be NULL)
 */
static inline void mitch_ticker_decode(uint64_t ticker_id, MitchTickerComponents *out)
{
    out->instr_type  = (MitchInstrType) ((ticker_id >> 60) & 0x0F);
    out->base_class  = (MitchAssetClass)((ticker_id >> 56) & 0x0F);
    out->base_id     = (uint16_t)        ((ticker_id >> 40) & 0xFFFF);
    out->quote_class = (MitchAssetClass)((ticker_id >> 36) & 0x0F);
    out->quote_id    = (uint16_t)        ((ticker_id >> 20) & 0xFFFF);
    out->sub_type    = (uint32_t)         (ticker_id        & 0xFFFFF);
}

/* ============================================================
 * U48 HELPERS
 * ============================================================ */

/**
 * Read a 6-byte little-endian unsigned integer into uint64_t.
 *
 * @param bytes  Pointer to 6 bytes
 * @return value as uint64_t (upper 2 bytes always 0)
 */
static inline uint64_t mitch_u48_read(const uint8_t bytes[6])
{
    uint64_t v = 0;
    memcpy(&v, bytes, 6);
#if defined(__BYTE_ORDER__) && __BYTE_ORDER__ == __ORDER_BIG_ENDIAN__
    v = __builtin_bswap64(v) >> 16;
#endif
    return v;
}

/**
 * Write a uint64_t value as a 6-byte little-endian integer.
 *
 * @param value  Value to write (must fit in 48 bits)
 * @param bytes  Destination buffer of at least 6 bytes
 */
static inline void mitch_u48_write(uint64_t value, uint8_t bytes[6])
{
    memcpy(bytes, &value, 6);
}

/* ============================================================
 * PACK / UNPACK - Header
 * ============================================================ */

/**
 * Pack a MitchHeader into the first 16 bytes of dst.
 *
 * @param hdr  Source header
 * @param dst  Destination buffer (must be >= 16 bytes)
 */
static inline void mitch_header_pack(const MitchHeader *hdr, uint8_t *dst)
{
    memcpy(dst, hdr, sizeof(MitchHeader));
}

/**
 * Unpack 16 bytes of wire data into a MitchHeader.
 *
 * @param src  Source buffer (must be >= 16 bytes)
 * @param hdr  Destination header struct
 */
static inline void mitch_header_unpack(const uint8_t *src, MitchHeader *hdr)
{
    memcpy(hdr, src, sizeof(MitchHeader));
}

/** Extract ASCII message type from type_provider field. */
static inline uint8_t mitch_header_msg_type(const MitchHeader *h) {
    return mitch_wire_to_ascii(h->type_provider & 0x0F);
}

/** Extract provider ID from type_provider field. */
static inline uint16_t mitch_header_provider_id(const MitchHeader *h) {
    return h->type_provider >> 4;
}

/** Read timestamp as 16us ticks since 2010-01-01. */
static inline uint64_t mitch_header_get_timestamp(const MitchHeader *h) {
    return mitch_u48_read(h->timestamp);
}

/** Write timestamp (16us ticks since 2010-01-01) into header. */
static inline void mitch_header_set_timestamp(MitchHeader *h, uint64_t ticks) {
    mitch_u48_write(ticks, h->timestamp);
}

/** Initialize a header from components. */
static inline void mitch_header_init(MitchHeader *h, uint8_t msg_type,
                                      uint16_t provider_id, uint64_t ts,
                                      uint8_t count) {
    h->type_provider = (mitch_ascii_to_wire(msg_type) & 0x0F) | (provider_id << 4);
    mitch_header_set_timestamp(h, ts);
    h->count = count;
    h->flags = 0;
    h->sequence = 0;
    memset(h->_reserved, 0, 4);
}

/* ============================================================
 * PACK / UNPACK - Tick
 * ============================================================ */

/**
 * Pack a MitchTick into exactly 32 bytes at dst.
 *
 * @param tick  Source tick
 * @param dst   Destination buffer (must be >= 32 bytes)
 */
static inline void mitch_tick_pack(const MitchTick *tick, uint8_t *dst)
{
    memcpy(dst, tick, sizeof(MitchTick));
}

/**
 * Unpack 32 bytes of wire data into a MitchTick.
 *
 * @param src   Source buffer (must be >= 32 bytes)
 * @param tick  Destination tick struct
 */
static inline void mitch_tick_unpack(const uint8_t *src, MitchTick *tick)
{
    memcpy(tick, src, sizeof(MitchTick));
}

/** Return mid price: (bid + ask) / 2. */
static inline double mitch_tick_mid(const MitchTick *tick)
{
    return (tick->bid_price + tick->ask_price) * 0.5;
}

/** Return bid-ask spread: ask - bid. */
static inline double mitch_tick_spread(const MitchTick *tick)
{
    return tick->ask_price - tick->bid_price;
}

/* ============================================================
 * PACK / UNPACK - Trade
 * ============================================================ */

/**
 * Pack a MitchTrade into exactly 24 bytes at dst.
 *
 * @param trade  Source trade
 * @param dst    Destination buffer (must be >= 24 bytes)
 */
static inline void mitch_trade_pack(const MitchTrade *trade, uint8_t *dst)
{
    memcpy(dst, trade, sizeof(MitchTrade));
}

/**
 * Unpack 24 bytes of wire data into a MitchTrade.
 *
 * @param src    Source buffer (must be >= 24 bytes)
 * @param trade  Destination trade struct
 */
static inline void mitch_trade_unpack(const uint8_t *src, MitchTrade *trade)
{
    memcpy(trade, src, sizeof(MitchTrade));
}

/** Return true if the trade is a buy. */
static inline bool mitch_trade_is_buy(const MitchTrade *trade)
{
    return trade->side == MITCH_SIDE_BUY;
}

/** Return true if the trade is a sell. */
static inline bool mitch_trade_is_sell(const MitchTrade *trade)
{
    return trade->side == MITCH_SIDE_SELL;
}

/* ============================================================
 * PACK / UNPACK - Order
 * ============================================================ */

/**
 * Pack a MitchOrder into exactly 32 bytes at dst.
 *
 * @param order  Source order
 * @param dst    Destination buffer (must be >= 32 bytes)
 */
static inline void mitch_order_pack(const MitchOrder *order, uint8_t *dst)
{
    memcpy(dst, order, sizeof(MitchOrder));
}

/**
 * Unpack 32 bytes of wire data into a MitchOrder.
 *
 * @param src    Source buffer (must be >= 32 bytes)
 * @param order  Destination order struct
 */
static inline void mitch_order_unpack(const uint8_t *src, MitchOrder *order)
{
    memcpy(order, src, sizeof(MitchOrder));
}

/** Extract order side from type_and_side byte. */
static inline MitchOrderSide mitch_order_side(const MitchOrder *order)
{
    return (MitchOrderSide)(order->type_and_side & 0x01);
}

/** Extract order type from type_and_side byte (bits [7:1]). */
static inline MitchOrderType mitch_order_type(const MitchOrder *order)
{
    return (MitchOrderType)((order->type_and_side >> 1) & 0x7F);
}

/** Build type_and_side byte from type and side enums. */
static inline uint8_t mitch_order_type_and_side(MitchOrderType t, MitchOrderSide s)
{
    return (uint8_t)(((uint8_t)t << 1) | ((uint8_t)s & 0x01));
}

/** Return order expiry as uint64_t (milliseconds since epoch). */
static inline uint64_t mitch_order_expiry_ms(const MitchOrder *order)
{
    return mitch_u48_read(order->expiry);
}

/** Write expiry (milliseconds since epoch) into order. */
static inline void mitch_order_set_expiry(MitchOrder *order, uint64_t ms)
{
    mitch_u48_write(ms, order->expiry);
}

/** Return true if this is a buy order. */
static inline bool mitch_order_is_buy(const MitchOrder *order)
{
    return mitch_order_side(order) == MITCH_SIDE_BUY;
}

/** Return true if this is a sell order. */
static inline bool mitch_order_is_sell(const MitchOrder *order)
{
    return mitch_order_side(order) == MITCH_SIDE_SELL;
}

/* ============================================================
 * PACK / UNPACK - Index
 * ============================================================ */

/**
 * Pack a MitchIndex into exactly 40 bytes at dst.
 *
 * @param idx  Source index
 * @param dst  Destination buffer (must be >= 40 bytes)
 */
static inline void mitch_index_pack(const MitchIndex *idx, uint8_t *dst)
{
    memcpy(dst, idx, sizeof(MitchIndex));
}

/**
 * Unpack 40 bytes of wire data into a MitchIndex.
 *
 * @param src  Source buffer (must be >= 40 bytes)
 * @param idx  Destination index struct
 */
static inline void mitch_index_unpack(const uint8_t *src, MitchIndex *idx)
{
    memcpy(idx, src, sizeof(MitchIndex));
}

/** Return true if index confidence is above the given threshold. */
static inline bool mitch_index_is_confident(const MitchIndex *idx, uint8_t min_confidence)
{
    return idx->confidence >= min_confidence;
}

/* ============================================================
 * PACK / UNPACK - Bin
 * ============================================================ */

/**
 * Pack a MitchBin into exactly 8 bytes at dst.
 *
 * @param bin  Source bin
 * @param dst  Destination buffer (must be >= 8 bytes)
 */
static inline void mitch_bin_pack(const MitchBin *bin, uint8_t *dst)
{
    memcpy(dst, bin, sizeof(MitchBin));
}

/**
 * Unpack 8 bytes of wire data into a MitchBin.
 *
 * @param src  Source buffer (must be >= 8 bytes)
 * @param bin  Destination bin struct
 */
static inline void mitch_bin_unpack(const uint8_t *src, MitchBin *bin)
{
    memcpy(bin, src, sizeof(MitchBin));
}

/* ============================================================
 * PACK / UNPACK - OrderBook
 * ============================================================ */

/**
 * Pack a MitchOrderBook into exactly 2072 bytes at dst.
 *
 * @param ob   Source order book
 * @param dst  Destination buffer (must be >= 2072 bytes)
 */
static inline void mitch_order_book_pack(const MitchOrderBook *ob, uint8_t *dst)
{
    memcpy(dst, ob, sizeof(MitchOrderBook));
}

/**
 * Unpack 2072 bytes of wire data into a MitchOrderBook.
 *
 * @param src  Source buffer (must be >= 2072 bytes)
 * @param ob   Destination order book struct
 */
static inline void mitch_order_book_unpack(const uint8_t *src, MitchOrderBook *ob)
{
    memcpy(ob, src, sizeof(MitchOrderBook));
}

/* ============================================================
 * CHANNEL ID UTILITIES
 *
 * 32-bit channel ID: [market_provider:16][message_type:8][padding:8]
 * ============================================================ */

/**
 * Generate a 32-bit channel ID.
 *
 * @param market_provider_id  Market provider ID (0-65535)
 * @param message_type        MITCH message type char ('s','t','o','i','b')
 * @return 32-bit channel ID
 */
static inline uint32_t mitch_channel_id(uint16_t market_provider_id, MitchMessageType message_type)
{
    return ((uint32_t)market_provider_id << 16)
         | ((uint32_t)(uint8_t)message_type << 8);
}

/**
 * Extract market provider ID from a channel ID.
 *
 * @param channel_id  32-bit channel ID
 * @return market provider ID
 */
static inline uint16_t mitch_channel_provider(uint32_t channel_id)
{
    return (uint16_t)(channel_id >> 16);
}

/**
 * Extract message type from a channel ID.
 *
 * @param channel_id  32-bit channel ID
 * @return message type byte
 */
static inline uint8_t mitch_channel_msg_type(uint32_t channel_id)
{
    return (uint8_t)(channel_id >> 8);
}

/* ============================================================
 * KNOWN MARKET PROVIDER IDS
 * ============================================================ */
#define MITCH_PROVIDER_BINANCE   101
#define MITCH_PROVIDER_BINGX     111
#define MITCH_PROVIDER_BITGET    141
#define MITCH_PROVIDER_BITMART   161
#define MITCH_PROVIDER_BITSTAMP  181
#define MITCH_PROVIDER_BITUNIX   191
#define MITCH_PROVIDER_BULLISH   251
#define MITCH_PROVIDER_BYBIT     261
#define MITCH_PROVIDER_COINBASE  341
#define MITCH_PROVIDER_CRYPTOCOM 391
#define MITCH_PROVIDER_GATE      561
#define MITCH_PROVIDER_GROVEX    583
#define MITCH_PROVIDER_HTX       641
#define MITCH_PROVIDER_KRAKEN    721
#define MITCH_PROVIDER_KUCOIN    741
#define MITCH_PROVIDER_LBANK     761
#define MITCH_PROVIDER_MEXC      821
#define MITCH_PROVIDER_OKX       911
#define MITCH_PROVIDER_TOOBIT   1181
#define MITCH_PROVIDER_UPBIT    1251
#define MITCH_PROVIDER_WHITEBIT 1281
#define MITCH_PROVIDER_XT       1301

/* ============================================================
 * MESSAGE SIZE CONSTANTS
 * ============================================================ */
#define MITCH_SIZE_HEADER     16
#define MITCH_SIZE_TICK       32
#define MITCH_SIZE_TRADE      24
#define MITCH_SIZE_ORDER      32
#define MITCH_SIZE_INDEX      40
#define MITCH_SIZE_BIN         8
#define MITCH_SIZE_ORDER_BOOK 2072
#define MITCH_SIZE_BAR        128
