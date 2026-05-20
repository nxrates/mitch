/**
 * mitch.hpp - MITCH Protocol C++17 Implementation
 *
 * Single-header, C++17, no external dependencies.
 * All structures use little-endian byte order and pack(1) layout.
 * Pack/unpack operations return std::optional and use span-based buffers.
 *
 * Binary layout reference (all little-endian):
 *   MitchHeader 16 bytes
 *   Tick        32 bytes
 *   Trade       24 bytes
 *   Order       32 bytes
 *   Index       40 bytes
 *   Bin          8 bytes
 *   OrderBook 2072 bytes
 *   Bar         96 bytes
 */

#pragma once

#include <array>
#include <cstdint>
#include <cstring>
#include <optional>
#include <span>

namespace mitch {

/* ============================================================
 * ENUMS
 * ============================================================ */

/** MITCH message type codes (ASCII). */
enum class MessageType : uint8_t {
    Tick      = 's', ///< 115 - Tick/quote snapshot
    Trade     = 't', ///< 116 - Trade execution
    Order     = 'o', ///< 111 - Order lifecycle
    Index     = 'i', ///< 105 - Index aggregated
    OrderBook = 'b', ///<  98 - Order book snapshot
    Bar       = 'k', ///< 107 - Bar/candlestick
};

/** Wire code (compact numeric type on the wire). */
enum class WireCode : uint8_t {
    Trade     = 1,
    Order     = 2,
    Tick      = 3,
    Index     = 4,
    OrderBook = 5,
    Bar       = 6,
};

/** Order side. */
enum class OrderSide : uint8_t {
    Buy  = 0,
    Sell = 1,
};

/** Order type (stored in bits [7:1] of type_and_side). */
enum class OrderType : uint8_t {
    Market = 0,
    Limit  = 1,
    Stop   = 2,
    Cancel = 3,
};

/** Asset class (4-bit value in ticker ID). */
enum class AssetClass : uint8_t {
    EQ  = 0,  ///< Equities
    CB  = 1,  ///< Corporate bonds
    SD  = 2,  ///< Sovereign debt
    FX  = 3,  ///< Forex
    CM  = 4,  ///< Commodities
    RE  = 5,  ///< Real estate
    CR  = 6,  ///< Crypto assets
    PM  = 7,  ///< Private markets
    CL  = 8,  ///< Collectibles
    IN  = 9,  ///< Infrastructure
    IP  = 10, ///< Indices/products
    SP  = 11, ///< Structured products
    CE  = 12, ///< Cash equivalents
    LR  = 13, ///< Loans/receivables
};

/** Instrument type (4-bit value, ticker ID bits [63:60]). */
enum class InstrType : uint8_t {
    Spot   = 0,
    Fut    = 1,
    Fwd    = 2,
    Swap   = 3,
    Perp   = 4,
    Cfd    = 5,
    Call   = 6,
    Put    = 7,
    Digi   = 8,
    Bar    = 9,
    War    = 10,
    Pred   = 11,
    Fund   = 12,
    Struct = 13,
};

/** Order book bin aggregation method. */
enum class BinAgg : uint8_t {
    Trilinear   = 0,
    Lingaussian = 1,
    Bilingeo    = 2,
    Lingeoflat  = 3,
};

/* ============================================================
 * WIRE CODE MAPPING
 * ============================================================ */

constexpr uint8_t wireToAscii(uint8_t code) {
    constexpr uint8_t tbl[] = {0, 't', 'o', 's', 'i', 'b', 'k'};
    return code <= 6 ? tbl[code] : 0;
}

constexpr uint8_t asciiToWire(uint8_t ch) {
    switch (ch) {
        case 't': return 1; case 'o': return 2; case 's': return 3;
        case 'i': return 4; case 'b': return 5; case 'k': return 6;
        default:  return 0;
    }
}

/* ============================================================
 * SIZE CONSTANTS
 * ============================================================ */
inline constexpr std::size_t kHeaderSize    =   16;
inline constexpr std::size_t kTickSize      =   32;
inline constexpr std::size_t kTradeSize     =   24;
inline constexpr std::size_t kOrderSize     =   32;
inline constexpr std::size_t kIndexSize     =   40;
inline constexpr std::size_t kBinSize       =    8;
inline constexpr std::size_t kOrderBookSize = 2072;
inline constexpr std::size_t kBarSize       =   96;

/* ============================================================
 * INTERNAL: portable little-endian read/write helpers
 * ============================================================ */
namespace detail {

template <typename T>
static T read_le(const uint8_t *p) noexcept
{
    T v;
    std::memcpy(&v, p, sizeof(T));
    // On big-endian hosts we would bswap here; MITCH targets LE hosts.
    return v;
}

template <typename T>
static void write_le(uint8_t *p, T v) noexcept
{
    std::memcpy(p, &v, sizeof(T));
}

/// Read a 6-byte little-endian u48 into uint64_t.
static inline uint64_t read_u48(const uint8_t *p) noexcept
{
    uint64_t v = 0;
    std::memcpy(&v, p, 6);
    return v;
}

/// Write uint64_t into a 6-byte little-endian u48 (upper bits ignored).
static inline void write_u48(uint8_t *p, uint64_t v) noexcept
{
    std::memcpy(p, &v, 6);
}

} // namespace detail

/* ============================================================
 * WIRE STRUCTURES  (packed layout, 1-byte alignment)
 * ============================================================ */

#pragma pack(push, 1)

/**
 * @brief 16-byte MITCH message header.
 *
 * Wire layout:
 *   [0..1]   type_provider : u16 LE  - [3:0]=wire_code, [15:4]=provider_id
 *   [2..7]   timestamp     : u48 LE  - 16µs ticks since 2010-01-01T00:00:00Z
 *   [8]      count         : u8      - batch entry count (1-255)
 *   [9]      flags         : u8      - [1:0]=version(0), [7:2]=reserved
 *   [10..11] sequence      : u16 LE  - per-stream gap detection
 *   [12..15] _reserved     : [u8; 4]
 */
struct Header {
    uint16_t type_provider;  ///< [3:0]=wire_code, [15:4]=provider_id
    uint8_t  timestamp[6];   ///< u48 LE: 16µs ticks since 2010-01-01
    uint8_t  count;          ///< batch entry count (1-255)
    uint8_t  flags;          ///< [1:0]=version(0), [7:2]=reserved
    uint16_t sequence;       ///< per-stream gap detection
    uint8_t  _reserved[4];

    [[nodiscard]] uint8_t  msgType()    const noexcept { return wireToAscii(type_provider & 0x0F); }
    [[nodiscard]] uint16_t providerId() const noexcept { return type_provider >> 4; }

    [[nodiscard]] uint64_t getTimestamp() const noexcept
    {
        uint64_t v = 0;
        std::memcpy(&v, timestamp, 6);
        return v;
    }

    void setTimestamp(uint64_t ts) noexcept { std::memcpy(timestamp, &ts, 6); }

    void init(uint8_t msg_type, uint16_t provider_id, uint64_t ts, uint8_t cnt) noexcept
    {
        type_provider = (asciiToWire(msg_type) & 0x0F) | (provider_id << 4);
        setTimestamp(ts);
        count = cnt;
        flags = 0;
        sequence = 0;
        std::memset(_reserved, 0, 4);
    }

    /// Pack this header into exactly 16 bytes.
    [[nodiscard]] std::array<uint8_t, 16> pack() const noexcept
    {
        std::array<uint8_t, 16> buf{};
        std::memcpy(buf.data(), this, 16);
        return buf;
    }

    /// Unpack 16 bytes into a Header; returns nullopt if span is too short.
    [[nodiscard]] static std::optional<Header> unpack(std::span<const uint8_t> src) noexcept
    {
        if (src.size() < 16) return std::nullopt;
        Header h{};
        std::memcpy(&h, src.data(), 16);
        return h;
    }
};
static_assert(sizeof(Header) == 16, "Header must be 16 bytes");

/**
 * @brief 32-byte tick/quote snapshot.
 *
 * Wire layout:
 *   [0..7]   ticker_id  : u64 LE
 *   [8..15]  bid_price  : f64 LE
 *   [16..23] ask_price  : f64 LE
 *   [24..27] bid_volume : u32 LE
 *   [28..31] ask_volume : u32 LE
 */
struct Tick {
    uint64_t ticker_id;
    double   bid_price;
    double   ask_price;
    uint32_t bid_volume;
    uint32_t ask_volume;

    /// Mid price: (bid + ask) / 2.
    [[nodiscard]] double mid() const noexcept
    {
        return (bid_price + ask_price) * 0.5;
    }

    /// Bid-ask spread: ask - bid.
    [[nodiscard]] double spread() const noexcept
    {
        return ask_price - bid_price;
    }

    /// Pack into exactly 32 bytes.
    [[nodiscard]] std::array<uint8_t, 32> pack() const noexcept
    {
        std::array<uint8_t, 32> buf;
        std::memcpy(buf.data(), this, 32);
        return buf;
    }

    /// Unpack 32 bytes; returns nullopt if span is too short.
    [[nodiscard]] static std::optional<Tick> unpack(std::span<const uint8_t> src) noexcept
    {
        if (src.size() < 32) return std::nullopt;
        Tick t;
        std::memcpy(&t, src.data(), 32);
        return t;
    }
};
static_assert(sizeof(Tick) == 32, "Tick must be 32 bytes");

/**
 * @brief 24-byte trade execution record.
 *
 * Wire layout:
 *   [0..7]   ticker_id : u64 LE
 *   [8..15]  price     : f64 LE
 *   [16..19] volume    : u32 LE
 *   [20..22] trade_id  : u24 LE (3 bytes, max 16_777_215)
 *   [23]     side      : u8  (0=Buy, 1=Sell)
 */
struct Trade {
    uint64_t ticker_id;
    double   price;
    uint32_t volume;
    uint8_t  trade_id[3]; ///< u24 little-endian
    uint8_t  side;

    [[nodiscard]] bool is_buy()  const noexcept { return side == static_cast<uint8_t>(OrderSide::Buy);  }
    [[nodiscard]] bool is_sell() const noexcept { return side == static_cast<uint8_t>(OrderSide::Sell); }

    /// Read trade_id as uint32_t (u24, upper byte always 0).
    [[nodiscard]] uint32_t get_trade_id() const noexcept
    {
        return static_cast<uint32_t>(trade_id[0])
             | (static_cast<uint32_t>(trade_id[1]) << 8)
             | (static_cast<uint32_t>(trade_id[2]) << 16);
    }

    /// Write trade_id from uint32_t (must fit in 24 bits).
    void set_trade_id(uint32_t id) noexcept
    {
        trade_id[0] = static_cast<uint8_t>(id);
        trade_id[1] = static_cast<uint8_t>(id >> 8);
        trade_id[2] = static_cast<uint8_t>(id >> 16);
    }

    /// Pack into exactly 24 bytes.
    [[nodiscard]] std::array<uint8_t, 24> pack() const noexcept
    {
        std::array<uint8_t, 24> buf;
        std::memcpy(buf.data(), this, 24);
        return buf;
    }

    /// Unpack 24 bytes; returns nullopt if span is too short.
    [[nodiscard]] static std::optional<Trade> unpack(std::span<const uint8_t> src) noexcept
    {
        if (src.size() < 24) return std::nullopt;
        Trade t;
        std::memcpy(&t, src.data(), 24);
        return t;
    }
};
static_assert(sizeof(Trade) == 24, "Trade must be 24 bytes");

/**
 * @brief 32-byte order lifecycle event.
 *
 * Wire layout:
 *   [0..7]   ticker_id     : u64 LE
 *   [8..11]  order_id      : u32 LE
 *   [12..19] price         : f64 LE
 *   [20..23] quantity      : u32 LE
 *   [24]     type_and_side : u8  bits[7:1]=order_type, bit[0]=side
 *   [25..30] expiry        : [u8; 6] - u48 LE ms since epoch
 *   [31]     padding       : u8
 */
struct Order {
    uint64_t ticker_id;
    uint32_t order_id;
    double   price;
    uint32_t quantity;
    uint8_t  type_and_side;
    uint8_t  expiry[6]; ///< u48 LE milliseconds since epoch
    uint8_t  _pad;

    [[nodiscard]] OrderSide side() const noexcept
    {
        return static_cast<OrderSide>(type_and_side & 0x01);
    }

    [[nodiscard]] OrderType order_type() const noexcept
    {
        return static_cast<OrderType>((type_and_side >> 1) & 0x7F);
    }

    /// Build type_and_side byte from explicit type and side.
    static constexpr uint8_t make_type_and_side(OrderType t, OrderSide s) noexcept
    {
        return static_cast<uint8_t>((static_cast<uint8_t>(t) << 1) | static_cast<uint8_t>(s));
    }

    [[nodiscard]] bool is_buy()  const noexcept { return side() == OrderSide::Buy;  }
    [[nodiscard]] bool is_sell() const noexcept { return side() == OrderSide::Sell; }

    /// Expiry as milliseconds since epoch.
    [[nodiscard]] uint64_t expiry_ms() const noexcept
    {
        return detail::read_u48(expiry);
    }

    void set_expiry(uint64_t ms) noexcept
    {
        detail::write_u48(expiry, ms);
    }

    /// Pack into exactly 32 bytes.
    [[nodiscard]] std::array<uint8_t, 32> pack() const noexcept
    {
        std::array<uint8_t, 32> buf;
        std::memcpy(buf.data(), this, 32);
        return buf;
    }

    /// Unpack 32 bytes; returns nullopt if span is too short.
    [[nodiscard]] static std::optional<Order> unpack(std::span<const uint8_t> src) noexcept
    {
        if (src.size() < 32) return std::nullopt;
        Order o;
        std::memcpy(&o, src.data(), 32);
        return o;
    }
};
static_assert(sizeof(Order) == 32, "Order must be 32 bytes");

/**
 * @brief 40-byte aggregated index snapshot.
 *
 * Wire layout:
 *   [0..7]   ticker_id  : u64 LE
 *   [8..15]  bid        : f64 LE
 *   [16..23] ask        : f64 LE
 *   [24..27] vbid       : u32 LE
 *   [28..31] vask       : u32 LE
 *   [32..33] ci         : u16 LE  (confidence interval, micro basis points)
 *   [34..35] tick_count : u16 LE
 *   [36]     confidence : u8
 *   [37]     accepted   : u8
 *   [38]     rejected   : u8
 *   [39]     padding    : u8
 */
struct Index {
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

    /// Returns true if confidence is at or above the given threshold.
    [[nodiscard]] bool is_confident(uint8_t min_confidence) const noexcept
    {
        return confidence >= min_confidence;
    }

    /// Spread: ask - bid.
    [[nodiscard]] double spread() const noexcept
    {
        return ask - bid;
    }

    /// Pack into exactly 40 bytes.
    [[nodiscard]] std::array<uint8_t, 40> pack() const noexcept
    {
        std::array<uint8_t, 40> buf;
        std::memcpy(buf.data(), this, 40);
        return buf;
    }

    /// Unpack 40 bytes; returns nullopt if span is too short.
    [[nodiscard]] static std::optional<Index> unpack(std::span<const uint8_t> src) noexcept
    {
        if (src.size() < 40) return std::nullopt;
        Index i;
        std::memcpy(&i, src.data(), 40);
        return i;
    }
};
static_assert(sizeof(Index) == 40, "Index must be 40 bytes");

/**
 * @brief 8-byte order book price bin.
 *
 * Wire layout:
 *   [0..3] order_count : u32 LE
 *   [4..7] volume      : u32 LE
 */
struct Bin {
    uint32_t order_count;
    uint32_t volume;

    /// Pack into exactly 8 bytes.
    [[nodiscard]] std::array<uint8_t, 8> pack() const noexcept
    {
        std::array<uint8_t, 8> buf;
        std::memcpy(buf.data(), this, 8);
        return buf;
    }

    /// Unpack 8 bytes; returns nullopt if span is too short.
    [[nodiscard]] static std::optional<Bin> unpack(std::span<const uint8_t> src) noexcept
    {
        if (src.size() < 8) return std::nullopt;
        Bin b;
        std::memcpy(&b, src.data(), 8);
        return b;
    }
};
static_assert(sizeof(Bin) == 8, "Bin must be 8 bytes");

/**
 * @brief 2072-byte aggregated order book snapshot.
 *
 * Wire layout:
 *   [0..7]       ticker_id      : u64 LE
 *   [8..15]      mid_price      : f64 LE
 *   [16]         bin_aggregator : u8
 *   [17..23]     padding        : [u8; 7]
 *   [24..1047]   bids           : [Bin; 128]
 *   [1048..2071] asks           : [Bin; 128]
 */
struct OrderBook {
    uint64_t ticker_id;
    double   mid_price;
    uint8_t  bin_aggregator;
    uint8_t  _pad[7];
    Bin      bids[128];
    Bin      asks[128];

    /// Pack into exactly 2072 bytes.
    [[nodiscard]] std::array<uint8_t, 2072> pack() const noexcept
    {
        std::array<uint8_t, 2072> buf;
        std::memcpy(buf.data(), this, 2072);
        return buf;
    }

    /// Unpack 2072 bytes; returns nullopt if span is too short.
    [[nodiscard]] static std::optional<OrderBook> unpack(std::span<const uint8_t> src) noexcept
    {
        if (src.size() < 2072) return std::nullopt;
        OrderBook ob;
        std::memcpy(&ob, src.data(), 2072);
        return ob;
    }
};
static_assert(sizeof(OrderBook) == 2072, "OrderBook must be 2072 bytes");

#pragma pack(pop)

/* ============================================================
 * TICKER ID ENCODE / DECODE
 *
 * Bit layout (64-bit):
 *   [63:60] InstrumentType  (4 bits)
 *   [59:56] BaseAssetClass  (4 bits)
 *   [55:40] BaseAssetID     (16 bits)
 *   [39:36] QuoteAssetClass (4 bits)
 *   [35:20] QuoteAssetID    (16 bits)
 *   [19:0]  SubType         (20 bits)
 * ============================================================ */

/// Decoded ticker ID components.
struct TickerComponents {
    InstrType   instr_type;
    AssetClass  base_class;
    uint16_t    base_id;
    AssetClass  quote_class;
    uint16_t    quote_id;
    uint32_t    sub_type;
};

/**
 * @brief Encode a 64-bit ticker ID from its components.
 *
 * @param instr_type   Instrument type  (InstrType enum)
 * @param base_class   Base asset class (AssetClass enum)
 * @param base_id      Base asset ID    (0-65535)
 * @param quote_class  Quote asset class(AssetClass enum)
 * @param quote_id     Quote asset ID   (0-65535)
 * @param sub_type     Sub-type         (0-1048575)
 * @return 64-bit ticker ID
 */
[[nodiscard]] constexpr uint64_t ticker_encode(
    InstrType   instr_type,
    AssetClass  base_class,
    uint16_t    base_id,
    AssetClass  quote_class,
    uint16_t    quote_id,
    uint32_t    sub_type = 0) noexcept
{
    return (static_cast<uint64_t>(instr_type)  << 60)
         | (static_cast<uint64_t>(base_class)  << 56)
         | (static_cast<uint64_t>(base_id)     << 40)
         | (static_cast<uint64_t>(quote_class) << 36)
         | (static_cast<uint64_t>(quote_id)    << 20)
         | (static_cast<uint64_t>(sub_type & 0xFFFFF));
}

/**
 * @brief Decode a 64-bit ticker ID into its components.
 *
 * @param ticker_id  Raw 64-bit ticker ID
 * @return TickerComponents struct
 */
[[nodiscard]] constexpr TickerComponents ticker_decode(uint64_t ticker_id) noexcept
{
    return TickerComponents{
        .instr_type  = static_cast<InstrType> ((ticker_id >> 60) & 0x0F),
        .base_class  = static_cast<AssetClass>((ticker_id >> 56) & 0x0F),
        .base_id     = static_cast<uint16_t>  ((ticker_id >> 40) & 0xFFFF),
        .quote_class = static_cast<AssetClass>((ticker_id >> 36) & 0x0F),
        .quote_id    = static_cast<uint16_t>  ((ticker_id >> 20) & 0xFFFF),
        .sub_type    = static_cast<uint32_t>   (ticker_id        & 0xFFFFF),
    };
}

/* ============================================================
 * CHANNEL ID UTILITIES
 *
 * 32-bit layout: [market_provider:16][message_type:8][padding:8]
 * ============================================================ */

/**
 * @brief Generate a 32-bit channel ID for pub/sub routing.
 *
 * @param provider_id  Market provider ID (0-65535)
 * @param msg_type     MITCH message type
 * @return 32-bit channel ID
 */
[[nodiscard]] constexpr uint32_t channel_id(uint16_t provider_id, MessageType msg_type) noexcept
{
    return (static_cast<uint32_t>(provider_id)                      << 16)
         | (static_cast<uint32_t>(static_cast<uint8_t>(msg_type))   <<  8);
}

/// Extract market provider ID from a channel ID.
[[nodiscard]] constexpr uint16_t channel_provider(uint32_t cid) noexcept
{
    return static_cast<uint16_t>(cid >> 16);
}

/// Extract message type from a channel ID.
[[nodiscard]] constexpr MessageType channel_msg_type(uint32_t cid) noexcept
{
    return static_cast<MessageType>(static_cast<uint8_t>(cid >> 8));
}

/* ============================================================
 * KNOWN MARKET PROVIDER IDS
 * ============================================================ */
inline constexpr uint16_t kProviderBinance   = 101;
inline constexpr uint16_t kProviderBingX     = 111;
inline constexpr uint16_t kProviderBitget    = 141;
inline constexpr uint16_t kProviderBitMart   = 161;
inline constexpr uint16_t kProviderBitstamp  = 181;
inline constexpr uint16_t kProviderBitunix   = 191;
inline constexpr uint16_t kProviderBullish   = 251;
inline constexpr uint16_t kProviderBybit     = 261;
inline constexpr uint16_t kProviderCoinbase  = 341;
inline constexpr uint16_t kProviderCryptoCom = 391;
inline constexpr uint16_t kProviderGate      = 561;
inline constexpr uint16_t kProviderGroveX    = 583;
inline constexpr uint16_t kProviderHTX       = 641;
inline constexpr uint16_t kProviderKraken    = 721;
inline constexpr uint16_t kProviderKuCoin    = 741;
inline constexpr uint16_t kProviderLBank     = 761;
inline constexpr uint16_t kProviderMEXC      = 821;
inline constexpr uint16_t kProviderOKX       = 911;
inline constexpr uint16_t kProviderToobit    = 1181;
inline constexpr uint16_t kProviderUpbit     = 1251;
inline constexpr uint16_t kProviderWhiteBIT  = 1281;
inline constexpr uint16_t kProviderXT        = 1301;

} // namespace mitch
