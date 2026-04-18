/**
 * Mitch.java - MITCH Protocol Java 11+ Implementation
 *
 * Single-file, pure stdlib, java.nio.ByteBuffer with LITTLE_ENDIAN byte order.
 * All pack/unpack operations use explicit offsets into a ByteBuffer.
 *
 * Binary layout reference (all little-endian):
 *   Header    16 bytes
 *   Tick      32 bytes
 *   Trade     24 bytes
 *   Order     32 bytes
 *   Index     40 bytes
 *   Bin        8 bytes
 *   Bar      128 bytes
 *   OrderBook  2072 bytes
 *
 * Usage:
 *   Mitch.Tick tick = new Mitch.Tick(...);
 *   byte[] wire = Mitch.Tick.pack(tick);
 *   Mitch.Tick decoded = Mitch.Tick.unpack(wire);
 */
package io.mitch;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;

/**
 * Namespace class for the MITCH binary protocol.
 * All nested types are public static.
 */
public final class Mitch {

    private Mitch() {}

    // =========================================================
    // Size constants
    // =========================================================

    /** Wire size of a Header in bytes. */
    public static final int SIZE_HEADER     =   16;
    /** Wire size of a Tick body in bytes. */
    public static final int SIZE_TICK       =   32;
    /** Wire size of a Trade body in bytes. */
    public static final int SIZE_TRADE      =   24;
    /** Wire size of an Order body in bytes. */
    public static final int SIZE_ORDER      =   32;
    /** Wire size of an Index body in bytes. */
    public static final int SIZE_INDEX      =   40;
    /** Wire size of a Bin in bytes. */
    public static final int SIZE_BIN        =    8;
    /** Wire size of a Bar body in bytes. */
    public static final int SIZE_BAR        =  128;
    /** Wire size of an OrderBook body in bytes. */
    public static final int SIZE_ORDER_BOOK = 2072;

    // =========================================================
    // Enums
    // =========================================================

    /** MITCH message type codes (ASCII). */
    public enum MessageType {
        TICK      ((byte)'s'),   // 115
        TRADE     ((byte)'t'),   // 116
        ORDER     ((byte)'o'),   // 111
        INDEX     ((byte)'i'),   // 105
        ORDER_BOOK((byte)'b'),   //  98
        BAR       ((byte)'k');   // 107

        public final byte value;
        MessageType(byte value) { this.value = value; }

        /** Look up MessageType from a raw byte value. */
        public static MessageType fromByte(byte b) {
            for (MessageType m : values()) {
                if (m.value == b) return m;
            }
            throw new IllegalArgumentException("Unknown MITCH message type: " + b);
        }
    }

    // =========================================================
    // Wire codes (compact numeric message type for header)
    // =========================================================

    public static final int WIRE_TRADE      = 1;
    public static final int WIRE_ORDER      = 2;
    public static final int WIRE_TICK       = 3;
    public static final int WIRE_INDEX      = 4;
    public static final int WIRE_ORDER_BOOK = 5;
    public static final int WIRE_BAR        = 6;

    /** Map a wire code (1-6) to its ASCII message type byte. */
    public static byte wireToAscii(int wireCode) {
        switch (wireCode) {
            case WIRE_TRADE:      return (byte)'t';
            case WIRE_ORDER:      return (byte)'o';
            case WIRE_TICK:       return (byte)'s';
            case WIRE_INDEX:      return (byte)'i';
            case WIRE_ORDER_BOOK: return (byte)'b';
            case WIRE_BAR:        return (byte)'k';
            default: throw new IllegalArgumentException("Unknown wire code: " + wireCode);
        }
    }

    /** Map an ASCII message type byte to its wire code (1-6). */
    public static int asciiToWire(int ascii) {
        switch (ascii) {
            case 't': return WIRE_TRADE;
            case 'o': return WIRE_ORDER;
            case 's': return WIRE_TICK;
            case 'i': return WIRE_INDEX;
            case 'b': return WIRE_ORDER_BOOK;
            case 'k': return WIRE_BAR;
            default: throw new IllegalArgumentException("Unknown ASCII message type: " + (char)ascii);
        }
    }

    /** Message type constant for Bar ('k', 0x6B). */
    public static final byte MSG_BAR = (byte)'k';

    /** Order side. */
    public enum OrderSide {
        BUY (0),
        SELL(1);

        public final int value;
        OrderSide(int value) { this.value = value; }

        public static OrderSide fromInt(int v) {
            return v == 0 ? BUY : SELL;
        }
    }

    /** Order type (stored in bits [7:1] of type_and_side). */
    public enum OrderType {
        MARKET(0),
        LIMIT (1),
        STOP  (2),
        CANCEL(3);

        public final int value;
        OrderType(int value) { this.value = value; }

        public static OrderType fromInt(int v) {
            for (OrderType t : values()) if (t.value == v) return t;
            throw new IllegalArgumentException("Unknown order type: " + v);
        }
    }

    /** Asset class (4-bit value in ticker ID). */
    public enum AssetClass {
        EQ (0),   // Equities
        CB (1),   // Corporate bonds
        SD (2),   // Sovereign debt
        FX (3),   // Forex
        CM (4),   // Commodities
        RE (5),   // Real estate
        CR (6),   // Crypto assets
        PM (7),   // Private markets
        CL (8),   // Collectibles
        IN (9),   // Infrastructure
        IP (10),  // Indices/products
        SP (11),  // Structured products
        CE (12),  // Cash equivalents
        LR (13);  // Loans/receivables

        public final int value;
        AssetClass(int value) { this.value = value; }

        public static AssetClass fromInt(int v) {
            for (AssetClass a : values()) if (a.value == v) return a;
            throw new IllegalArgumentException("Unknown asset class: " + v);
        }
    }

    /** Instrument type (4-bit value, ticker ID bits [63:60]). */
    public enum InstrType {
        SPOT  (0),
        FUT   (1),
        FWD   (2),
        SWAP  (3),
        PERP  (4),
        CFD   (5),
        CALL  (6),
        PUT   (7),
        DIGI  (8),
        BAR   (9),
        WAR   (10),
        PRED  (11),
        FUND  (12),
        STRUCT(13);

        public final int value;
        InstrType(int value) { this.value = value; }

        public static InstrType fromInt(int v) {
            for (InstrType t : values()) if (t.value == v) return t;
            throw new IllegalArgumentException("Unknown instrument type: " + v);
        }
    }

    /** Order book bin aggregation method. */
    public enum BinAgg {
        TRILINEAR  (0),
        LINGAUSSIAN(1),
        BILINGEO   (2),
        LINGEOFLAT (3);

        public final int value;
        BinAgg(int value) { this.value = value; }

        public static BinAgg fromInt(int v) {
            for (BinAgg a : values()) if (a.value == v) return a;
            throw new IllegalArgumentException("Unknown bin aggregation: " + v);
        }
    }

    // =========================================================
    // Known market provider IDs
    // =========================================================

    public static final int PROVIDER_BINANCE   = 101;
    public static final int PROVIDER_BINGX     = 111;
    public static final int PROVIDER_BITGET    = 141;
    public static final int PROVIDER_BITMART   = 161;
    public static final int PROVIDER_BITSTAMP  = 181;
    public static final int PROVIDER_BITUNIX   = 191;
    public static final int PROVIDER_BULLISH   = 251;
    public static final int PROVIDER_BYBIT     = 261;
    public static final int PROVIDER_COINBASE  = 341;
    public static final int PROVIDER_CRYPTOCOM = 391;
    public static final int PROVIDER_GATE      = 561;
    public static final int PROVIDER_GROVEX    = 583;
    public static final int PROVIDER_HTX       = 641;
    public static final int PROVIDER_KRAKEN    = 721;
    public static final int PROVIDER_KUCOIN    = 741;
    public static final int PROVIDER_LBANK     = 761;
    public static final int PROVIDER_MEXC      = 821;
    public static final int PROVIDER_OKX       = 911;
    public static final int PROVIDER_TOOBIT    = 1181;
    public static final int PROVIDER_UPBIT     = 1251;
    public static final int PROVIDER_WHITEBIT  = 1281;
    public static final int PROVIDER_XT        = 1301;

    // =========================================================
    // Internal helpers
    // =========================================================

    /**
     * Read a 6-byte little-endian u48 from buf at the given offset.
     * Returns the value as a long (upper 2 bytes always 0).
     */
    private static long readU48(ByteBuffer buf, int offset) {
        long v = 0L;
        for (int i = 0; i < 6; i++) {
            v |= (long)(buf.get(offset + i) & 0xFF) << (8 * i);
        }
        return v;
    }

    /**
     * Write a long value as a 6-byte little-endian u48 into buf at offset.
     * Upper 2 bytes of value are ignored.
     */
    private static void writeU48(ByteBuffer buf, int offset, long value) {
        for (int i = 0; i < 6; i++) {
            buf.put(offset + i, (byte)(value >> (8 * i)));
        }
    }

    /** Allocate a little-endian ByteBuffer of the given size. */
    private static ByteBuffer leBuffer(int size) {
        return ByteBuffer.allocate(size).order(ByteOrder.LITTLE_ENDIAN);
    }

    // =========================================================
    // Header
    // =========================================================

    /**
     * 16-byte MITCH message header.
     *
     * Wire layout:
     *   [0..1]   typeProvider   : u16 LE - bits[3:0]=wire_code, bits[15:4]=provider_id
     *   [2..7]   timestampTicks : u48 LE - 16µs ticks since 2010-01-01T00:00:00Z
     *   [8]      count          : u8
     *   [9]      flags          : u8
     *   [10..11] sequence       : u16 LE
     *   [12..15] reserved       : 4 bytes
     */
    public static final class Header {
        /** Raw type_provider field: bits[3:0]=wire_code, bits[15:4]=provider_id. */
        public final int typeProvider;   // u16 stored as int
        /** 16µs ticks since 2010-01-01T00:00:00Z (u48 range). */
        public final long timestampTicks;
        /** Number of body entries (1-255). */
        public final int count;
        /** Flags byte. */
        public final int flags;
        /** Sequence number (u16). */
        public final int sequence;

        public Header(int typeProvider, long timestampTicks, int count,
                       int flags, int sequence) {
            this.typeProvider   = typeProvider & 0xFFFF;
            this.timestampTicks = timestampTicks;
            this.count          = count & 0xFF;
            this.flags          = flags & 0xFF;
            this.sequence       = sequence & 0xFFFF;
        }

        /** Extract the ASCII message type byte from typeProvider (via wire code). */
        public byte msgType() {
            int wireCode = typeProvider & 0x0F;
            return wireToAscii(wireCode);
        }

        /** Extract the provider ID from typeProvider (bits [15:4]). */
        public int providerId() {
            return (typeProvider >> 4) & 0x0FFF;
        }

        /**
         * Create a Header from an ASCII message type, provider ID, and other fields.
         *
         * @param asciiType  ASCII message type byte (e.g. 's', 't', 'o', etc.)
         * @param providerId Market provider ID (0-4095)
         * @param ticks      Timestamp in 16µs ticks
         * @param count      Body entry count (1-255)
         * @param flags      Flags byte
         * @param sequence   Sequence number (u16)
         * @return new Header
         */
        public static Header create(byte asciiType, int providerId,
                                     long ticks, int count,
                                     int flags, int sequence) {
            int wireCode = asciiToWire(asciiType);
            int tp = ((providerId & 0x0FFF) << 4) | (wireCode & 0x0F);
            return new Header(tp, ticks, count, flags, sequence);
        }

        /**
         * Pack this header into exactly 16 bytes.
         *
         * @return byte array of length 16
         */
        public static byte[] pack(Header h) {
            ByteBuffer buf = leBuffer(SIZE_HEADER);
            buf.putShort(0, (short) h.typeProvider);
            writeU48(buf, 2, h.timestampTicks);
            buf.put(8, (byte) h.count);
            buf.put(9, (byte) h.flags);
            buf.putShort(10, (short) h.sequence);
            // [12..15] reserved = 0
            return buf.array();
        }

        /**
         * Unpack exactly 16 bytes into a Header.
         *
         * @param data byte array of at least 16 bytes
         * @return decoded Header
         * @throws IllegalArgumentException if data is too short
         */
        public static Header unpack(byte[] data) {
            if (data.length < SIZE_HEADER) {
                throw new IllegalArgumentException("Header requires " + SIZE_HEADER + " bytes");
            }
            ByteBuffer buf = ByteBuffer.wrap(data).order(ByteOrder.LITTLE_ENDIAN);
            return new Header(
                Short.toUnsignedInt(buf.getShort(0)),
                readU48(buf, 2),
                buf.get(8) & 0xFF,
                buf.get(9) & 0xFF,
                Short.toUnsignedInt(buf.getShort(10))
            );
        }

        @Override
        public String toString() {
            return "Header{type=" + (char)msgType() +
                   ", provider=" + providerId() +
                   ", ts=" + timestampTicks + "ticks, count=" + count +
                   ", flags=" + flags + ", seq=" + sequence + '}';
        }
    }

    // =========================================================
    // Tick
    // =========================================================

    /**
     * 32-byte tick/quote snapshot.
     *
     * Wire layout:
     *   [0..7]   tickerId  : u64 LE
     *   [8..15]  bidPrice  : f64 LE
     *   [16..23] askPrice  : f64 LE
     *   [24..27] bidVolume : u32 LE
     *   [28..31] askVolume : u32 LE
     */
    public static final class Tick {
        public final long   tickerId;
        public final double bidPrice;
        public final double askPrice;
        public final long   bidVolume;  // unsigned 32-bit stored as long
        public final long   askVolume;  // unsigned 32-bit stored as long

        public Tick(long tickerId, double bidPrice, double askPrice,
                    long bidVolume, long askVolume) {
            this.tickerId  = tickerId;
            this.bidPrice  = bidPrice;
            this.askPrice  = askPrice;
            this.bidVolume = bidVolume;
            this.askVolume = askVolume;
        }

        /** Mid price: (bid + ask) / 2. */
        public double mid() { return (bidPrice + askPrice) * 0.5; }

        /** Bid-ask spread: ask - bid. */
        public double spread() { return askPrice - bidPrice; }

        /**
         * Pack into exactly 32 bytes.
         *
         * @return byte array of length 32
         */
        public static byte[] pack(Tick t) {
            ByteBuffer buf = leBuffer(SIZE_TICK);
            buf.putLong  (0,  t.tickerId);
            buf.putDouble(8,  t.bidPrice);
            buf.putDouble(16, t.askPrice);
            buf.putInt   (24, (int) t.bidVolume);
            buf.putInt   (28, (int) t.askVolume);
            return buf.array();
        }

        /**
         * Unpack 32 bytes into a Tick.
         *
         * @param data byte array of at least 32 bytes
         * @return decoded Tick
         * @throws IllegalArgumentException if data is too short
         */
        public static Tick unpack(byte[] data) {
            if (data.length < SIZE_TICK) {
                throw new IllegalArgumentException("Tick requires " + SIZE_TICK + " bytes");
            }
            ByteBuffer buf = ByteBuffer.wrap(data).order(ByteOrder.LITTLE_ENDIAN);
            return new Tick(
                buf.getLong(0),
                buf.getDouble(8),
                buf.getDouble(16),
                Integer.toUnsignedLong(buf.getInt(24)),
                Integer.toUnsignedLong(buf.getInt(28))
            );
        }

        @Override
        public String toString() {
            return "Tick{ticker=" + tickerId + ", bid=" + bidPrice +
                   ", ask=" + askPrice + ", bidVol=" + bidVolume +
                   ", askVol=" + askVolume + '}';
        }
    }

    // =========================================================
    // Trade
    // =========================================================

    /**
     * 24-byte trade execution record.
     *
     * Wire layout:
     *   [0..7]   tickerId : u64 LE
     *   [8..15]  price    : f64 LE
     *   [16..19] volume   : u32 LE
     *   [20..22] tradeId  : u24 LE (3 bytes, max 16_777_215)
     *   [23]     side     : u8  (0=Buy, 1=Sell)
     */
    public static final class Trade {
        public final long      tickerId;
        public final double    price;
        public final long      volume;     // u32 as long
        public final long      tradeId;    // u24 as long (max 16_777_215)
        public final OrderSide side;

        public Trade(long tickerId, double price, long volume,
                     long tradeId, OrderSide side) {
            this.tickerId = tickerId;
            this.price    = price;
            this.volume   = volume;
            this.tradeId  = tradeId;
            this.side     = side;
        }

        /** Returns true if this is a buy trade. */
        public boolean isBuy()  { return side == OrderSide.BUY;  }
        /** Returns true if this is a sell trade. */
        public boolean isSell() { return side == OrderSide.SELL; }

        /**
         * Pack into exactly 24 bytes.
         *
         * @return byte array of length 24
         */
        public static byte[] pack(Trade t) {
            ByteBuffer buf = leBuffer(SIZE_TRADE);
            buf.putLong(0,  t.tickerId);
            buf.putDouble(8, t.price);
            buf.putInt(16, (int) t.volume);
            buf.put(20, (byte)(t.tradeId));
            buf.put(21, (byte)(t.tradeId >> 8));
            buf.put(22, (byte)(t.tradeId >> 16));
            buf.put(23, (byte) t.side.value);
            return buf.array();
        }

        /**
         * Unpack 24 bytes into a Trade.
         *
         * @param data byte array of at least 24 bytes
         * @return decoded Trade
         */
        public static Trade unpack(byte[] data) {
            if (data.length < SIZE_TRADE) {
                throw new IllegalArgumentException("Trade requires " + SIZE_TRADE + " bytes");
            }
            ByteBuffer buf = ByteBuffer.wrap(data).order(ByteOrder.LITTLE_ENDIAN);
            return new Trade(
                buf.getLong(0),
                buf.getDouble(8),
                Integer.toUnsignedLong(buf.getInt(16)),
                (long)(buf.get(20) & 0xFF) | ((long)(buf.get(21) & 0xFF) << 8) | ((long)(buf.get(22) & 0xFF) << 16),
                OrderSide.fromInt(buf.get(23) & 0xFF)
            );
        }

        @Override
        public String toString() {
            return "Trade{ticker=" + tickerId + ", price=" + price +
                   ", vol=" + volume + ", id=" + tradeId +
                   ", side=" + side + '}';
        }
    }

    // =========================================================
    // Order
    // =========================================================

    /**
     * 32-byte order lifecycle event.
     *
     * Wire layout:
     *   [0..7]   tickerId     : u64 LE
     *   [8..11]  orderId      : u32 LE
     *   [12..19] price        : f64 LE
     *   [20..23] quantity     : u32 LE
     *   [24]     typeAndSide  : u8  bits[7:1]=order_type, bit[0]=side
     *   [25..30] expiry       : [u8; 6] - u48 LE ms since epoch
     *   [31]     padding      : u8
     */
    public static final class Order {
        public final long      tickerId;
        public final long      orderId;     // u32 as long
        public final double    price;
        public final long      quantity;    // u32 as long
        /** Raw encoded byte: bits[7:1] = order type, bit[0] = side. */
        public final int       typeAndSide;
        /** Expiry in milliseconds since epoch (u48). */
        public final long      expiryMs;

        public Order(long tickerId, long orderId, double price,
                     long quantity, int typeAndSide, long expiryMs) {
            this.tickerId    = tickerId;
            this.orderId     = orderId;
            this.price       = price;
            this.quantity    = quantity;
            this.typeAndSide = typeAndSide;
            this.expiryMs    = expiryMs;
        }

        /** Convenience constructor accepting explicit type and side. */
        public Order(long tickerId, long orderId, double price, long quantity,
                     OrderType type, OrderSide side, long expiryMs) {
            this(tickerId, orderId, price, quantity,
                 makeTypeAndSide(type, side), expiryMs);
        }

        /** Build typeAndSide byte from explicit type and side. */
        public static int makeTypeAndSide(OrderType t, OrderSide s) {
            return ((t.value & 0x7F) << 1) | (s.value & 0x01);
        }

        /** Extract OrderSide from typeAndSide. */
        public OrderSide side() { return OrderSide.fromInt(typeAndSide & 0x01); }

        /** Extract OrderType from typeAndSide. */
        public OrderType type() { return OrderType.fromInt((typeAndSide >> 1) & 0x7F); }

        /** Returns true if this is a buy order. */
        public boolean isBuy()  { return side() == OrderSide.BUY;  }
        /** Returns true if this is a sell order. */
        public boolean isSell() { return side() == OrderSide.SELL; }

        /**
         * Pack into exactly 32 bytes.
         *
         * @return byte array of length 32
         */
        public static byte[] pack(Order o) {
            ByteBuffer buf = leBuffer(SIZE_ORDER);
            buf.putLong(0, o.tickerId);
            buf.putInt(8, (int) o.orderId);
            buf.putDouble(12, o.price);
            buf.putInt(20, (int) o.quantity);
            buf.put(24, (byte) o.typeAndSide);
            writeU48(buf, 25, o.expiryMs);
            // buf[31] = 0 (padding)
            return buf.array();
        }

        /**
         * Unpack 32 bytes into an Order.
         *
         * @param data byte array of at least 32 bytes
         * @return decoded Order
         */
        public static Order unpack(byte[] data) {
            if (data.length < SIZE_ORDER) {
                throw new IllegalArgumentException("Order requires " + SIZE_ORDER + " bytes");
            }
            ByteBuffer buf = ByteBuffer.wrap(data).order(ByteOrder.LITTLE_ENDIAN);
            return new Order(
                buf.getLong(0),
                Integer.toUnsignedLong(buf.getInt(8)),
                buf.getDouble(12),
                Integer.toUnsignedLong(buf.getInt(20)),
                buf.get(24) & 0xFF,
                readU48(buf, 25)
            );
        }

        @Override
        public String toString() {
            return "Order{ticker=" + tickerId + ", id=" + orderId +
                   ", price=" + price + ", qty=" + quantity +
                   ", type=" + type() + ", side=" + side() +
                   ", expiry=" + expiryMs + "ms}";
        }
    }

    // =========================================================
    // Index
    // =========================================================

    /**
     * 40-byte aggregated index snapshot.
     *
     * Wire layout:
     *   [0..7]   tickerId   : u64 LE
     *   [8..15]  bid        : f64 LE
     *   [16..23] ask        : f64 LE
     *   [24..27] vBid       : u32 LE
     *   [28..31] vAsk       : u32 LE
     *   [32..33] ci         : u16 LE  (confidence interval, micro basis points)
     *   [34..35] tickCount  : u16 LE
     *   [36]     confidence : u8
     *   [37]     accepted   : u8
     *   [38]     rejected   : u8
     *   [39]     padding    : u8
     */
    public static final class Index {
        public final long   tickerId;
        public final double bid;
        public final double ask;
        public final long   vBid;       // u32 as long
        public final long   vAsk;       // u32 as long
        public final int    ci;         // u16 (confidence interval, micro basis points)
        public final int    tickCount;  // u16
        public final int    confidence; // u8
        public final int    accepted;   // u8
        public final int    rejected;   // u8

        public Index(long tickerId, double bid, double ask,
                     long vBid, long vAsk, int ci, int tickCount,
                     int confidence, int accepted, int rejected) {
            this.tickerId   = tickerId;
            this.bid        = bid;
            this.ask        = ask;
            this.vBid       = vBid;
            this.vAsk       = vAsk;
            this.ci         = ci;
            this.tickCount  = tickCount;
            this.confidence = confidence;
            this.accepted   = accepted;
            this.rejected   = rejected;
        }

        /** Returns true if confidence >= minConfidence. */
        public boolean isConfident(int minConfidence) {
            return confidence >= minConfidence;
        }

        /** Spread: ask - bid. */
        public double spread() { return ask - bid; }

        /**
         * Pack into exactly 40 bytes (1 byte padding at end).
         *
         * @return byte array of length 40
         */
        public static byte[] pack(Index idx) {
            ByteBuffer buf = leBuffer(SIZE_INDEX);
            buf.putLong  (0,  idx.tickerId);
            buf.putDouble(8,  idx.bid);
            buf.putDouble(16, idx.ask);
            buf.putInt   (24, (int) idx.vBid);
            buf.putInt   (28, (int) idx.vAsk);
            buf.putShort (32, (short) idx.ci);
            buf.putShort (34, (short) idx.tickCount);
            buf.put      (36, (byte) idx.confidence);
            buf.put      (37, (byte) idx.accepted);
            buf.put      (38, (byte) idx.rejected);
            // [39] padding = 0
            return buf.array();
        }

        /**
         * Unpack 40 bytes into an Index.
         *
         * @param data byte array of at least 40 bytes
         * @return decoded Index
         */
        public static Index unpack(byte[] data) {
            if (data.length < SIZE_INDEX) {
                throw new IllegalArgumentException("Index requires " + SIZE_INDEX + " bytes");
            }
            ByteBuffer buf = ByteBuffer.wrap(data).order(ByteOrder.LITTLE_ENDIAN);
            return new Index(
                buf.getLong(0),
                buf.getDouble(8),
                buf.getDouble(16),
                Integer.toUnsignedLong(buf.getInt(24)),
                Integer.toUnsignedLong(buf.getInt(28)),
                Short.toUnsignedInt(buf.getShort(32)),
                Short.toUnsignedInt(buf.getShort(34)),
                buf.get(36) & 0xFF,
                buf.get(37) & 0xFF,
                buf.get(38) & 0xFF
            );
        }
    }

    // =========================================================
    // Bin
    // =========================================================

    /**
     * 8-byte order book price-level bin.
     *
     * Wire layout:
     *   [0..3] orderCount : u32 LE
     *   [4..7] volume     : u32 LE
     */
    public static final class Bin {
        public final long orderCount; // u32 as long
        public final long volume;     // u32 as long

        public Bin(long orderCount, long volume) {
            this.orderCount = orderCount;
            this.volume     = volume;
        }

        /**
         * Pack into exactly 8 bytes.
         *
         * @return byte array of length 8
         */
        public static byte[] pack(Bin b) {
            ByteBuffer buf = leBuffer(SIZE_BIN);
            buf.putInt(0, (int) b.orderCount);
            buf.putInt(4, (int) b.volume);
            return buf.array();
        }

        /**
         * Unpack 8 bytes into a Bin.
         *
         * @param data byte array of at least 8 bytes
         * @return decoded Bin
         */
        public static Bin unpack(byte[] data) {
            if (data.length < SIZE_BIN) {
                throw new IllegalArgumentException("Bin requires " + SIZE_BIN + " bytes");
            }
            ByteBuffer buf = ByteBuffer.wrap(data).order(ByteOrder.LITTLE_ENDIAN);
            return new Bin(
                Integer.toUnsignedLong(buf.getInt(0)),
                Integer.toUnsignedLong(buf.getInt(4))
            );
        }
    }

    // =========================================================
    // OrderBook
    // =========================================================

    /**
     * 2072-byte aggregated order book snapshot.
     *
     * Wire layout:
     *   [0..7]       tickerId      : u64 LE
     *   [8..15]      midPrice      : f64 LE
     *   [16]         binAggregator : u8
     *   [17..23]     padding       : 7 bytes
     *   [24..1047]   bids          : [Bin; 128]
     *   [1048..2071] asks          : [Bin; 128]
     */
    public static final class OrderBook {
        public final long   tickerId;
        public final double midPrice;
        public final BinAgg binAggregator;
        public final Bin[]  bids; // length must be 128
        public final Bin[]  asks; // length must be 128

        public OrderBook(long tickerId, double midPrice, BinAgg binAggregator,
                         Bin[] bids, Bin[] asks) {
            if (bids.length != 128) throw new IllegalArgumentException("bids must have 128 entries");
            if (asks.length != 128) throw new IllegalArgumentException("asks must have 128 entries");
            this.tickerId      = tickerId;
            this.midPrice      = midPrice;
            this.binAggregator = binAggregator;
            this.bids          = bids;
            this.asks          = asks;
        }

        /**
         * Pack into exactly 2072 bytes.
         *
         * @return byte array of length 2072
         */
        public static byte[] pack(OrderBook ob) {
            ByteBuffer buf = leBuffer(SIZE_ORDER_BOOK);
            buf.putLong  (0,  ob.tickerId);
            buf.putDouble(8,  ob.midPrice);
            buf.put      (16, (byte) ob.binAggregator.value);
            // [17..23] padding = 0
            int off = 24;
            for (Bin b : ob.bids) {
                buf.putInt(off,     (int) b.orderCount);
                buf.putInt(off + 4, (int) b.volume);
                off += 8;
            }
            for (Bin b : ob.asks) {
                buf.putInt(off,     (int) b.orderCount);
                buf.putInt(off + 4, (int) b.volume);
                off += 8;
            }
            return buf.array();
        }

        /**
         * Unpack 2072 bytes into an OrderBook.
         *
         * @param data byte array of at least 2072 bytes
         * @return decoded OrderBook
         */
        public static OrderBook unpack(byte[] data) {
            if (data.length < SIZE_ORDER_BOOK) {
                throw new IllegalArgumentException(
                    "OrderBook requires " + SIZE_ORDER_BOOK + " bytes");
            }
            ByteBuffer buf = ByteBuffer.wrap(data).order(ByteOrder.LITTLE_ENDIAN);
            long   tickerId      = buf.getLong(0);
            double midPrice      = buf.getDouble(8);
            BinAgg binAggregator = BinAgg.fromInt(buf.get(16) & 0xFF);
            Bin[]  bids = new Bin[128];
            Bin[]  asks = new Bin[128];
            int off = 24;
            for (int i = 0; i < 128; i++) {
                bids[i] = new Bin(
                    Integer.toUnsignedLong(buf.getInt(off)),
                    Integer.toUnsignedLong(buf.getInt(off + 4))
                );
                off += 8;
            }
            for (int i = 0; i < 128; i++) {
                asks[i] = new Bin(
                    Integer.toUnsignedLong(buf.getInt(off)),
                    Integer.toUnsignedLong(buf.getInt(off + 4))
                );
                off += 8;
            }
            return new OrderBook(tickerId, midPrice, binAggregator, bids, asks);
        }
    }

    // =========================================================
    // TickerID encode / decode
    //
    // Bit layout (64-bit):
    //   [63:60] InstrumentType  (4 bits)
    //   [59:56] BaseAssetClass  (4 bits)
    //   [55:40] BaseAssetID     (16 bits)
    //   [39:36] QuoteAssetClass (4 bits)
    //   [35:20] QuoteAssetID    (16 bits)
    //   [19:0]  SubType         (20 bits)
    // =========================================================

    /** Decoded components of a ticker ID. */
    public static final class TickerComponents {
        public final InstrType  instrType;
        public final AssetClass baseClass;
        public final int        baseId;     // 0-65535
        public final AssetClass quoteClass;
        public final int        quoteId;    // 0-65535
        public final int        subType;    // 0-1048575

        public TickerComponents(InstrType instrType, AssetClass baseClass, int baseId,
                                AssetClass quoteClass, int quoteId, int subType) {
            this.instrType  = instrType;
            this.baseClass  = baseClass;
            this.baseId     = baseId;
            this.quoteClass = quoteClass;
            this.quoteId    = quoteId;
            this.subType    = subType;
        }
    }

    /**
     * Encode a 64-bit ticker ID from its components.
     *
     * @param instrType   Instrument type
     * @param baseClass   Base asset class
     * @param baseId      Base asset ID (0-65535)
     * @param quoteClass  Quote asset class
     * @param quoteId     Quote asset ID (0-65535)
     * @param subType     Sub-type (0-1048575)
     * @return 64-bit ticker ID
     */
    public static long tickerEncode(InstrType instrType, AssetClass baseClass, int baseId,
                                    AssetClass quoteClass, int quoteId, int subType) {
        return ((long)(instrType.value  & 0x0F) << 60)
             | ((long)(baseClass.value  & 0x0F) << 56)
             | ((long)(baseId    & 0xFFFF)       << 40)
             | ((long)(quoteClass.value & 0x0F)  << 36)
             | ((long)(quoteId   & 0xFFFF)        << 20)
             | ((long)(subType   & 0xFFFFF));
    }

    /**
     * Decode a 64-bit ticker ID into its components.
     *
     * @param tickerId  Raw 64-bit ticker ID
     * @return TickerComponents struct
     */
    public static TickerComponents tickerDecode(long tickerId) {
        return new TickerComponents(
            InstrType.fromInt ((int)((tickerId >> 60) & 0x0F)),
            AssetClass.fromInt((int)((tickerId >> 56) & 0x0F)),
            (int)((tickerId >> 40) & 0xFFFFL),
            AssetClass.fromInt((int)((tickerId >> 36) & 0x0F)),
            (int)((tickerId >> 20) & 0xFFFFL),
            (int) (tickerId        & 0xFFFFFL)
        );
    }

    // =========================================================
    // Channel ID utilities
    //
    // 32-bit layout: [market_provider:16][message_type:8][padding:8]
    // =========================================================

    /**
     * Generate a 32-bit channel ID for pub/sub routing.
     *
     * @param providerId  Market provider ID (0-65535)
     * @param msgType     MITCH message type
     * @return 32-bit channel ID (as int; use Integer.toUnsignedLong if needed)
     */
    public static int channelId(int providerId, MessageType msgType) {
        return ((providerId & 0xFFFF) << 16) | ((msgType.value & 0xFF) << 8);
    }

    /**
     * Extract the market provider ID from a channel ID.
     *
     * @param channelId  32-bit channel ID
     * @return market provider ID
     */
    public static int channelProvider(int channelId) {
        return (channelId >>> 16) & 0xFFFF;
    }

    /**
     * Extract the MessageType from a channel ID.
     *
     * @param channelId  32-bit channel ID
     * @return MessageType
     */
    public static MessageType channelMsgType(int channelId) {
        return MessageType.fromByte((byte)((channelId >>> 8) & 0xFF));
    }
}
