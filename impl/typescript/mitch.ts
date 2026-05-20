/**
 * MITCH Protocol TypeScript Implementation
 *
 * Data structures and pack/unpack for the MITCH (Moded ITCH) binary protocol.
 * Optimized for ultra-low latency financial market data transmission.
 *
 * Binary layout reference (all little-endian):
 *   Header     16 bytes
 *   Tick       32 bytes
 *   Trade      24 bytes
 *   Order      32 bytes
 *   Index      40 bytes
 *   Bin         8 bytes
 *   OrderBook 2072 bytes
 *   Bar        96 bytes
 *
 * Features:
 * - Trade, Order, Tick, OrderBook, and Index message types
 * - 8-byte Ticker ID encoding with asset class support
 * - 32-bit Channel ID system for pub/sub filtering
 * - Little-endian serialization for cross-platform compatibility
 * - pack() / unpack() exported for every body type
 */

// =============================================================================
// CONSTANTS AND ENUMS
// =============================================================================

/** MITCH message type codes (ASCII) */
export const MessageType = {
  TICK:       's'.charCodeAt(0), // 115
  TRADE:      't'.charCodeAt(0), // 116
  ORDER:      'o'.charCodeAt(0), // 111
  INDEX:      'i'.charCodeAt(0), // 105
  ORDER_BOOK: 'b'.charCodeAt(0), //  98
  BAR:        'k'.charCodeAt(0), // 107
} as const;

export type MessageTypeValue = (typeof MessageType)[keyof typeof MessageType];

/** Message type lookup for reverse mapping */
export const MessageTypeChar: Record<number, string> = {
  [MessageType.TICK]:       's',
  [MessageType.TRADE]:      't',
  [MessageType.ORDER]:      'o',
  [MessageType.INDEX]:      'i',
  [MessageType.ORDER_BOOK]: 'b',
  [MessageType.BAR]:        'k',
};

/** Compact wire codes for header typeProvider field */
export const WireCode = {
  TRADE:      1,
  ORDER:      2,
  TICK:       3,
  INDEX:      4,
  ORDER_BOOK: 5,
  BAR:        6,
} as const;

export type WireCodeValue = (typeof WireCode)[keyof typeof WireCode];

const wireToAsciiTable = [0, MessageType.TRADE, MessageType.ORDER, MessageType.TICK, MessageType.INDEX, MessageType.ORDER_BOOK, MessageType.BAR];

export function wireToAscii(code: number): number {
  return code <= 6 ? wireToAsciiTable[code] : 0;
}

export function asciiToWire(ch: number): number {
  switch (ch) {
    case MessageType.TRADE: return 1;
    case MessageType.ORDER: return 2;
    case MessageType.TICK:  return 3;
    case MessageType.INDEX: return 4;
    case MessageType.ORDER_BOOK: return 5;
    case MessageType.BAR:   return 6;
    default: return 0;
  }
}

/** Order type (stored in bits [7:1] of typeAndSide) */
export enum OrderType {
  MARKET = 0,
  LIMIT  = 1,
  STOP   = 2,
  CANCEL = 3,
}

/** Order / trade side */
export enum OrderSide {
  BUY  = 0,
  SELL = 1,
}

/** Asset classes for Ticker ID encoding */
export enum AssetClass {
  EQ  = 0x0,  // Equities
  CB  = 0x1,  // Corporate bonds
  SD  = 0x2,  // Sovereign debt
  FX  = 0x3,  // Forex
  CM  = 0x4,  // Commodities
  RE  = 0x5,  // Real estate
  CR  = 0x6,  // Crypto assets
  PM  = 0x7,  // Private markets
  CL  = 0x8,  // Collectibles
  IN  = 0x9,  // Infrastructure
  IP  = 0xA,  // Indices/products
  SP  = 0xB,  // Structured products
  CE  = 0xC,  // Cash equivalents
  LR  = 0xD,  // Loans/receivables
  // Legacy aliases kept for backwards compat
  EQUITIES            = 0x0,
  CORPORATE_BONDS     = 0x1,
  SOVEREIGN_DEBT      = 0x2,
  FOREX               = 0x3,
  COMMODITIES         = 0x4,
  REAL_ESTATE         = 0x5,
  CRYPTO_ASSETS       = 0x6,
  PRIVATE_MARKETS     = 0x7,
  COLLECTIBLES        = 0x8,
  INFRASTRUCTURE      = 0x9,
  INDICES             = 0xA,
  STRUCTURED_PRODUCTS = 0xB,
  CASH_EQUIVALENTS    = 0xC,
  LOANS_RECEIVABLES   = 0xD,
}

/** Instrument types for Ticker ID encoding */
export enum InstrumentType {
  SPOT                = 0x0,
  FUTURE              = 0x1,
  FORWARD             = 0x2,
  SWAP                = 0x3,
  PERPETUAL_SWAP      = 0x4,
  CFD                 = 0x5,
  CALL_OPTION         = 0x6,
  PUT_OPTION          = 0x7,
  DIGITAL_OPTION      = 0x8,
  BARRIER_OPTION      = 0x9,
  WARRANT             = 0xA,
  PREDICTION_CONTRACT = 0xB,
  STRUCTURED_PRODUCT  = 0xC,
  FUND                = 0xD,
}

/** Order book bin aggregation method */
export enum BinAggregator {
  TRILINEAR   = 0,
  LINGAUSSIAN = 1,
  BILINGEO    = 2,
  LINGEOFLAT  = 3,
  // Legacy aliases
  DEFAULT_LINGAUSSIAN = 1,
  DEFAULT_LINGEOFLAT  = 3,
  DEFAULT_BILINGEO    = 2,
  DEFAULT_TRILINEAR   = 0,
}

/** Index calculation types */
export enum IndexType {
  MID   = 0x00,
  BBID  = 0x01,
  BASK  = 0x02,
  WBID  = 0x03,
  WASK  = 0x04,
  MBID  = 0x05,
  MASK  = 0x06,
  VWAP  = 0x07,
  TWAP  = 0x08,
  LAST  = 0x09,
  OPEN  = 0x0A,
  HIGH  = 0x0B,
  LOW   = 0x0C,
  CLOSE = 0x0D,
}

// =============================================================================
// MESSAGE SIZE CONSTANTS
// =============================================================================

/** Wire sizes in bytes for each MITCH message body type. */
export const MESSAGE_SIZES = {
  HEADER:          16,
  TICK:            32,
  TRADE:           24,
  ORDER:           32,
  INDEX:           40,
  BIN:              8,
  ORDER_BOOK:    2072,
  BAR:             96,
  // Legacy keys
  ORDER_BOOK_HEADER: 32,
  ORDER_BOOK_VOLUME:  4,
} as const;

export const SIZE_HEADER = 16;
export const SIZE_BAR = 96;

// =============================================================================
// KNOWN MARKET PROVIDER IDs
// =============================================================================

export const Provider = {
  BINANCE:    101,
  BINGX:      111,
  BITGET:     141,
  BITMART:    161,
  BITSTAMP:   181,
  BITUNIX:    191,
  BULLISH:    251,
  BYBIT:      261,
  COINBASE:   341,
  CRYPTOCOM:  391,
  GATE:       561,
  GROVEX:     583,
  HTX:        641,
  KRAKEN:     721,
  KUCOIN:     741,
  LBANK:      761,
  MEXC:       821,
  OKX:        911,
  TOOBIT:    1181,
  UPBIT:     1251,
  WHITEBIT:  1281,
  XT:        1301,
} as const;

// =============================================================================
// CORE DATA STRUCTURES
// =============================================================================

/**
 * MITCH unified message header (16 bytes).
 *
 * Wire layout:
 *   [0..1]   typeProvider : u16 LE - [3:0]=wire_code, [15:4]=provider_id
 *   [2..7]   timestamp   : u48 LE - 16µs ticks since 2010-01-01T00:00:00Z
 *   [8]      count       : u8
 *   [9]      flags       : u8
 *   [10..11] sequence    : u16 LE
 *   [12..15] reserved    : 4 bytes (zero)
 */
export interface MitchHeader {
  /** u16: [3:0]=wire_code, [15:4]=provider_id */
  typeProvider: number;
  /** u48 as bigint: 16µs ticks since 2010-01-01T00:00:00Z */
  timestamp: bigint;
  /** u8: number of body entries (1-255) */
  count: number;
  /** u8: flags */
  flags: number;
  /** u16: sequence number */
  sequence: number;
}

/** Helper functions for timestamp conversion */
export const TimestampUtils = {
  /** Convert bigint (16µs ticks since 2010-01-01) to 6-byte LE Uint8Array */
  timestampToBytes(ticks: bigint): Uint8Array {
    const bytes = new Uint8Array(6);
    for (let i = 0; i < 6; i++) {
      bytes[i] = Number((ticks >> BigInt(i * 8)) & 0xFFn);
    }
    return bytes;
  },

  /** Convert 6-byte LE Uint8Array to bigint (16µs ticks since 2010-01-01) */
  bytesToTimestamp(bytes: Uint8Array): bigint {
    let v = 0n;
    for (let i = 0; i < 6; i++) {
      v |= BigInt(bytes[i]) << BigInt(i * 8);
    }
    return v;
  },
};

/**
 * Tick/quote snapshot body (32 bytes).
 *
 * Wire layout:
 *   [0..7]   tickerId  : u64 LE
 *   [8..15]  bidPrice  : f64 LE
 *   [16..23] askPrice  : f64 LE
 *   [24..27] bidVolume : u32 LE
 *   [28..31] askVolume : u32 LE
 */
export interface Tick {
  tickerId:  bigint;  // u64
  bidPrice:  number;  // f64
  askPrice:  number;  // f64
  bidVolume: number;  // u32
  askVolume: number;  // u32
}

/** Mid price: (bid + ask) / 2. */
export function tickMid(tick: Tick): number {
  return (tick.bidPrice + tick.askPrice) * 0.5;
}

/** Bid-ask spread: ask - bid. */
export function tickSpread(tick: Tick): number {
  return tick.askPrice - tick.bidPrice;
}

/**
 * Trade execution body (24 bytes).
 *
 * Wire layout:
 *   [0..7]   tickerId : u64 LE
 *   [8..15]  price    : f64 LE
 *   [16..19] volume   : u32 LE
 *   [20..22] tradeId  : u24 LE (3 bytes, max 16_777_215)
 *   [23]     side     : u8  (0=Buy, 1=Sell)
 */
export interface Trade {
  tickerId: bigint;  // u64
  price:    number;  // f64
  volume:   number;  // u32
  tradeId:  number;  // u24 (max 16_777_215)
  side:     OrderSide; // u8
}

/**
 * Order lifecycle event body (32 bytes).
 *
 * Wire layout:
 *   [0..7]   tickerId     : u64 LE
 *   [8..11]  orderId      : u32 LE
 *   [12..19] price        : f64 LE
 *   [20..23] quantity     : u32 LE
 *   [24]     typeAndSide  : u8  bits[7:1]=order_type, bit[0]=side
 *   [25..30] expiry       : u48 LE - milliseconds since epoch
 *   [31]     padding      : u8
 */
export interface Order {
  tickerId:    bigint;  // u64
  orderId:     number;  // u32
  price:       number;  // f64
  quantity:    number;  // u32
  /** Combined byte: bits[7:1] = OrderType, bit[0] = OrderSide. */
  typeAndSide: number;  // u8
  /** Expiry in milliseconds since epoch (u48). */
  expiryMs:    bigint;
  // 1 byte padding
}

/** Build typeAndSide byte from explicit type and side. */
export function makeTypeAndSide(type: OrderType, side: OrderSide): number {
  return ((type & 0x7F) << 1) | (side & 0x01);
}

/** Extract OrderType from a typeAndSide byte. */
export function orderTypeFromByte(typeAndSide: number): OrderType {
  return (typeAndSide >> 1) & 0x7F;
}

/** Extract OrderSide from a typeAndSide byte. */
export function orderSideFromByte(typeAndSide: number): OrderSide {
  return typeAndSide & 0x01;
}

/**
 * Index aggregated snapshot body (40 bytes).
 *
 * Wire layout:
 *   [0..7]   tickerId   : u64 LE
 *   [8..15]  bid        : f64 LE
 *   [16..23] ask        : f64 LE
 *   [24..27] vbid       : u32 LE
 *   [28..31] vask       : u32 LE
 *   [32..33] ci         : u16 LE  (confidence interval, micro basis points)
 *   [34..35] tickCount  : u16 LE
 *   [36]     confidence : u8
 *   [37]     accepted   : u8
 *   [38]     rejected   : u8
 *   [39]     padding    : u8
 */
export interface Index {
  tickerId:   bigint;  // u64
  bid:        number;  // f64
  ask:        number;  // f64
  vbid:       number;  // u32
  vask:       number;  // u32
  ci:         number;  // u16 (confidence interval, micro basis points)
  tickCount:  number;  // u16
  confidence: number;  // u8
  accepted:   number;  // u8
  rejected:   number;  // u8
  // 1 byte padding
}

/**
 * Order book price-level bin (8 bytes).
 *
 * Wire layout:
 *   [0..3] count  : u32 LE
 *   [4..7] volume : u32 LE
 */
export interface Bin {
  count:  number;  // u32
  volume: number;  // u32
}

/**
 * Aggregated order book snapshot body (2072 bytes).
 *
 * Wire layout:
 *   [0..7]       tickerId      : u64 LE
 *   [8..15]      midPrice      : f64 LE
 *   [16]         binAggregator : u8
 *   [17..23]     padding       : 7 bytes
 *   [24..1047]   bids          : [Bin; 128]
 *   [1048..2071] asks          : [Bin; 128]
 */
export interface OptimizedOrderBook {
  tickerId:      bigint;          // u64
  midPrice:      number;          // f64
  binAggregator: BinAggregator;   // u8
  // 7 bytes padding
  bids: Bin[];  // exactly 128 entries
  asks: Bin[];  // exactly 128 entries
}

// =============================================================================
// MITCH MESSAGE CONTAINER
// =============================================================================

/** Complete MITCH message with header and typed body entries */
export type MitchMessage =
  | { header: MitchHeader; body: Tick[] }
  | { header: MitchHeader; body: Trade[] }
  | { header: MitchHeader; body: Order[] }
  | { header: MitchHeader; body: Index[] }
  | { header: MitchHeader; body: OptimizedOrderBook[] };

// =============================================================================
// CHANNEL ID SYSTEM
// =============================================================================

/** Channel ID components for pub/sub filtering */
export interface ChannelIdComponents {
  marketProviderId: number;  // u16: market provider ID
  messageType: string;       // char: MITCH message type
  padding: number;           // u8: reserved (0)
}

/** Channel ID utilities for pub/sub systems */
export class ChannelId {
  /**
   * Generate a 32-bit channel ID.
   * Format: [market_provider:16][message_type:8][padding:8]
   */
  static generate(marketProviderId: number, messageType: string): number {
    if (marketProviderId > 0xFFFF) {
      throw new Error('Market provider ID must fit in 16 bits');
    }
    return (marketProviderId << 16) | (messageType.charCodeAt(0) << 8);
  }

  /** Extract components from a 32-bit channel ID. */
  static extract(channelId: number): ChannelIdComponents {
    return {
      marketProviderId: (channelId >>> 16) & 0xFFFF,
      messageType:      String.fromCharCode((channelId >>> 8) & 0xFF),
      padding:          channelId & 0xFF,
    };
  }

  /** Validate channel ID format. */
  static validate(channelId: number): boolean {
    if (channelId < 0 || channelId > 0xFFFFFFFF) return false;
    const { messageType, padding } = ChannelId.extract(channelId);
    return ['t', 'o', 's', 'b', 'i', 'k'].includes(messageType) && padding === 0;
  }

  /**
   * Generate channel pattern for pub/sub pattern matching.
   * Example: generatePattern(101, '*') -> "657*" for all Binance messages.
   */
  static generatePattern(marketProviderId: number, messageTypePattern: string): string {
    if (messageTypePattern === '*') {
      return (marketProviderId << 8).toString(16).toUpperCase() + '*';
    }
    return ChannelId.generate(marketProviderId, messageTypePattern).toString();
  }
}

// =============================================================================
// TICKER ID UTILITIES
// =============================================================================

/**
 * Ticker ID encoding utilities.
 *
 * Bit layout (64-bit):
 *   [63:60] InstrumentType  (4 bits)
 *   [59:56] BaseAssetClass  (4 bits)
 *   [55:40] BaseAssetID     (16 bits)
 *   [39:36] QuoteAssetClass (4 bits)
 *   [35:20] QuoteAssetID    (16 bits)
 *   [19:0]  SubType         (20 bits)
 */
export class TickerId {
  /**
   * Encode a 64-bit ticker ID from its components.
   *
   * @param instrumentType  Instrument type (InstrumentType enum)
   * @param baseClass       Base asset class (AssetClass enum)
   * @param baseId          Base asset ID (0-65535)
   * @param quoteClass      Quote asset class (AssetClass enum)
   * @param quoteId         Quote asset ID (0-65535)
   * @param subType         Sub-type (0-1048575)
   */
  static generate(
    instrumentType: InstrumentType,
    baseClass: AssetClass,
    baseId: number,
    quoteClass: AssetClass,
    quoteId: number,
    subType: number = 0,
  ): bigint {
    if (baseId > 0xFFFF || quoteId > 0xFFFF || subType > 0xFFFFF) {
      throw new Error('baseId/quoteId must fit in 16 bits; subType in 20 bits');
    }
    return (BigInt(instrumentType & 0x0F) << 60n)
         | (BigInt(baseClass      & 0x0F) << 56n)
         | (BigInt(baseId)                << 40n)
         | (BigInt(quoteClass     & 0x0F) << 36n)
         | (BigInt(quoteId)               << 20n)
         |  BigInt(subType & 0xFFFFF);
  }

  /** Decode a 64-bit ticker ID into its components. */
  static extract(tickerId: bigint): {
    instrumentType: InstrumentType;
    baseClass:      AssetClass;
    baseId:         number;
    quoteClass:     AssetClass;
    quoteId:        number;
    subType:        number;
  } {
    const instrNum  = Number((tickerId >> 60n) & 0xFn);
    const baseNum   = Number((tickerId >> 56n) & 0xFn);
    const baseId    = Number((tickerId >> 40n) & 0xFFFFn);
    const quoteNum  = Number((tickerId >> 36n) & 0xFn);
    const quoteId   = Number((tickerId >> 20n) & 0xFFFFn);
    const subType   = Number( tickerId         & 0xFFFFFn);
    return {
      instrumentType: instrNum  as InstrumentType,
      baseClass:      baseNum   as AssetClass,
      baseId,
      quoteClass:     quoteNum  as AssetClass,
      quoteId,
      subType,
    };
  }
}

// =============================================================================
// VALIDATION UTILITIES
// =============================================================================

/** Index confidence level descriptions */
export const confidenceLevel = {
  PERFECT:       100,
  HIGH:           80,
  MEDIUM:         60,
  LOW:            40,
  VERY_LOW:       20,
  NO_CONFIDENCE:   0,
} as const;

/** Validate index confidence score */
export function validateconfidence(confidence: number): boolean {
  return Number.isInteger(confidence) && confidence >= 0 && confidence <= 255;
}

/** Validate index type */
export function validateIndexType(indexType: number): boolean {
  return Object.values(IndexType).includes(indexType);
}

/** Channel ID examples for common exchanges */
export const CHANNEL_EXAMPLES = {
  BINANCE_TICKS:    ChannelId.generate(101,  's'),
  COINBASE_TRADES:  ChannelId.generate(341,  't'),
  KRAKEN_INDICES:   ChannelId.generate(721,  'i'),
} as const;

// =============================================================================
// INTERNAL HELPERS
// =============================================================================

/** Read a 6-byte LE u48 from a DataView and return it as bigint. */
function readU48(dv: DataView, offset: number): bigint {
  let v = 0n;
  for (let i = 0; i < 6; i++) {
    v |= BigInt(dv.getUint8(offset + i)) << BigInt(i * 8);
  }
  return v;
}

/** Write a bigint (u48 range) as 6 LE bytes into a DataView. */
function writeU48(dv: DataView, offset: number, value: bigint): void {
  for (let i = 0; i < 6; i++) {
    dv.setUint8(offset + i, Number((value >> BigInt(i * 8)) & 0xFFn));
  }
}

// =============================================================================
// PACK - individual body types (exported)
// =============================================================================

/**
 * Pack a MitchHeader into exactly 16 bytes.
 */
export function packHeader(h: MitchHeader): DataView {
  const dv = new DataView(new ArrayBuffer(SIZE_HEADER));
  dv.setUint16(0, h.typeProvider, true);
  writeU48(dv, 2, h.timestamp);
  dv.setUint8(8, h.count);
  dv.setUint8(9, h.flags);
  dv.setUint16(10, h.sequence, true);
  // bytes 12-15 reserved (zero)
  return dv;
}

/**
 * Unpack 16 bytes into a MitchHeader.
 * @throws RangeError if src is shorter than 16 bytes.
 */
export function unpackHeader(dv: DataView, offset = 0): MitchHeader {
  return {
    typeProvider: dv.getUint16(offset, true),
    timestamp:   readU48(dv, offset + 2),
    count:       dv.getUint8(offset + 8),
    flags:       dv.getUint8(offset + 9),
    sequence:    dv.getUint16(offset + 10, true),
  };
}

/** Extract the ASCII message type from a header. */
export function headerMsgType(h: MitchHeader): number {
  return wireToAscii(h.typeProvider & 0x0F);
}

/** Extract the provider ID from a header. */
export function headerProviderId(h: MitchHeader): number {
  return h.typeProvider >> 4;
}

/** Create a header with default flags=0, sequence=0. */
export function createHeader(msgType: number, providerId: number, timestamp: bigint, count: number): MitchHeader {
  return {
    typeProvider: (asciiToWire(msgType) & 0x0F) | (providerId << 4),
    timestamp,
    count,
    flags: 0,
    sequence: 0,
  };
}

/**
 * Pack a Tick body into exactly 32 bytes.
 */
export function packTick(t: Tick): Uint8Array {
  const buf = new Uint8Array(MESSAGE_SIZES.TICK);
  const dv  = new DataView(buf.buffer);
  dv.setBigUint64(0,  t.tickerId,  true);
  dv.setFloat64  (8,  t.bidPrice,  true);
  dv.setFloat64  (16, t.askPrice,  true);
  dv.setUint32   (24, t.bidVolume, true);
  dv.setUint32   (28, t.askVolume, true);
  return buf;
}

/**
 * Unpack 32 bytes into a Tick.
 * @throws RangeError if src is shorter than 32 bytes.
 */
export function unpackTick(src: Uint8Array): Tick {
  if (src.length < MESSAGE_SIZES.TICK) {
    throw new RangeError(`Tick requires ${MESSAGE_SIZES.TICK} bytes`);
  }
  const dv = new DataView(src.buffer, src.byteOffset, src.byteLength);
  return {
    tickerId:  dv.getBigUint64(0,  true),
    bidPrice:  dv.getFloat64  (8,  true),
    askPrice:  dv.getFloat64  (16, true),
    bidVolume: dv.getUint32   (24, true),
    askVolume: dv.getUint32   (28, true),
  };
}

/**
 * Pack a Trade body into exactly 24 bytes.
 */
export function packTrade(t: Trade): Uint8Array {
  const buf = new Uint8Array(MESSAGE_SIZES.TRADE);
  const dv  = new DataView(buf.buffer);
  dv.setBigUint64(0,  t.tickerId, true);
  dv.setFloat64  (8,  t.price,    true);
  dv.setUint32   (16, t.volume,   true);
  dv.setUint8    (20, t.tradeId & 0xFF);
  dv.setUint8    (21, (t.tradeId >> 8) & 0xFF);
  dv.setUint8    (22, (t.tradeId >> 16) & 0xFF);
  dv.setUint8    (23, t.side);
  return buf;
}

/**
 * Unpack 24 bytes into a Trade.
 * @throws RangeError if src is shorter than 24 bytes.
 */
export function unpackTrade(src: Uint8Array): Trade {
  if (src.length < MESSAGE_SIZES.TRADE) {
    throw new RangeError(`Trade requires ${MESSAGE_SIZES.TRADE} bytes`);
  }
  const dv = new DataView(src.buffer, src.byteOffset, src.byteLength);
  return {
    tickerId: dv.getBigUint64(0,  true),
    price:    dv.getFloat64  (8,  true),
    volume:   dv.getUint32   (16, true),
    tradeId:  dv.getUint8(20) | (dv.getUint8(21) << 8) | (dv.getUint8(22) << 16),
    side:     dv.getUint8    (23) as OrderSide,
  };
}

/**
 * Pack an Order body into exactly 32 bytes.
 */
export function packOrder(o: Order): Uint8Array {
  const buf = new Uint8Array(MESSAGE_SIZES.ORDER); // zero-initialised (padding = 0)
  const dv  = new DataView(buf.buffer);
  dv.setBigUint64(0,  o.tickerId,    true);
  dv.setUint32   (8,  o.orderId,     true);
  dv.setFloat64  (12, o.price,       true);
  dv.setUint32   (20, o.quantity,    true);
  dv.setUint8    (24, o.typeAndSide);
  writeU48       (dv, 25, o.expiryMs);
  // [31] = 0 (padding)
  return buf;
}

/**
 * Unpack 32 bytes into an Order.
 * @throws RangeError if src is shorter than 32 bytes.
 */
export function unpackOrder(src: Uint8Array): Order {
  if (src.length < MESSAGE_SIZES.ORDER) {
    throw new RangeError(`Order requires ${MESSAGE_SIZES.ORDER} bytes`);
  }
  const dv = new DataView(src.buffer, src.byteOffset, src.byteLength);
  return {
    tickerId:    dv.getBigUint64(0,  true),
    orderId:     dv.getUint32   (8,  true),
    price:       dv.getFloat64  (12, true),
    quantity:    dv.getUint32   (20, true),
    typeAndSide: dv.getUint8    (24),
    expiryMs:    readU48        (dv, 25),
  };
}

/**
 * Pack an Index body into exactly 40 bytes.
 */
export function packIndex(idx: Index): Uint8Array {
  const buf = new Uint8Array(MESSAGE_SIZES.INDEX); // zero-initialised (padding = 0)
  const dv  = new DataView(buf.buffer);
  dv.setBigUint64(0,  idx.tickerId,   true);
  dv.setFloat64  (8,  idx.bid,        true);
  dv.setFloat64  (16, idx.ask,        true);
  dv.setUint32   (24, idx.vbid,       true);
  dv.setUint32   (28, idx.vask,       true);
  dv.setUint16   (32, idx.ci,         true);
  dv.setUint16   (34, idx.tickCount,  true);
  dv.setUint8    (36, idx.confidence);
  dv.setUint8    (37, idx.accepted);
  dv.setUint8    (38, idx.rejected);
  // [39] = 0 (padding)
  return buf;
}

/**
 * Unpack 40 bytes into an Index.
 * @throws RangeError if src is shorter than 40 bytes.
 */
export function unpackIndex(src: Uint8Array): Index {
  if (src.length < MESSAGE_SIZES.INDEX) {
    throw new RangeError(`Index requires ${MESSAGE_SIZES.INDEX} bytes`);
  }
  const dv = new DataView(src.buffer, src.byteOffset, src.byteLength);
  return {
    tickerId:   dv.getBigUint64(0,  true),
    bid:        dv.getFloat64  (8,  true),
    ask:        dv.getFloat64  (16, true),
    vbid:       dv.getUint32   (24, true),
    vask:       dv.getUint32   (28, true),
    ci:         dv.getUint16   (32, true),
    tickCount:  dv.getUint16   (34, true),
    confidence: dv.getUint8    (36),
    accepted:   dv.getUint8    (37),
    rejected:   dv.getUint8    (38),
  };
}

/**
 * Pack a Bin into exactly 8 bytes.
 */
export function packBin(b: Bin): Uint8Array {
  const buf = new Uint8Array(MESSAGE_SIZES.BIN);
  const dv  = new DataView(buf.buffer);
  dv.setUint32(0, b.count,  true);
  dv.setUint32(4, b.volume, true);
  return buf;
}

/**
 * Unpack 8 bytes into a Bin.
 * @throws RangeError if src is shorter than 8 bytes.
 */
export function unpackBin(src: Uint8Array): Bin {
  if (src.length < MESSAGE_SIZES.BIN) {
    throw new RangeError(`Bin requires ${MESSAGE_SIZES.BIN} bytes`);
  }
  const dv = new DataView(src.buffer, src.byteOffset, src.byteLength);
  return { count: dv.getUint32(0, true), volume: dv.getUint32(4, true) };
}

/**
 * Pack an OptimizedOrderBook body into exactly 2072 bytes.
 * bids and asks must each have exactly 128 entries.
 */
export function packOrderBook(ob: OptimizedOrderBook): Uint8Array {
  const buf = new Uint8Array(MESSAGE_SIZES.ORDER_BOOK);
  const dv  = new DataView(buf.buffer);
  dv.setBigUint64(0,  ob.tickerId,      true);
  dv.setFloat64  (8,  ob.midPrice,      true);
  dv.setUint8    (16, ob.binAggregator);
  // [17..23] = 0 (padding)
  let off = 24;
  for (const bid of ob.bids) {
    dv.setUint32(off,     bid.count,  true);
    dv.setUint32(off + 4, bid.volume, true);
    off += 8;
  }
  for (const ask of ob.asks) {
    dv.setUint32(off,     ask.count,  true);
    dv.setUint32(off + 4, ask.volume, true);
    off += 8;
  }
  return buf;
}

/**
 * Unpack 2072 bytes into an OptimizedOrderBook.
 * @throws RangeError if src is shorter than 2072 bytes.
 */
export function unpackOrderBook(src: Uint8Array): OptimizedOrderBook {
  if (src.length < MESSAGE_SIZES.ORDER_BOOK) {
    throw new RangeError(`OrderBook requires ${MESSAGE_SIZES.ORDER_BOOK} bytes`);
  }
  const dv = new DataView(src.buffer, src.byteOffset, src.byteLength);
  const tickerId      = dv.getBigUint64(0, true);
  const midPrice      = dv.getFloat64(8,   true);
  const binAggregator = dv.getUint8(16) as BinAggregator;
  let off = 24;
  const bids: Bin[] = [];
  for (let i = 0; i < 128; i++) {
    bids.push({ count: dv.getUint32(off, true), volume: dv.getUint32(off + 4, true) });
    off += 8;
  }
  const asks: Bin[] = [];
  for (let i = 0; i < 128; i++) {
    asks.push({ count: dv.getUint32(off, true), volume: dv.getUint32(off + 4, true) });
    off += 8;
  }
  return { tickerId, midPrice, binAggregator, bids, asks };
}

// =============================================================================
// PACK / UNPACK - full MitchMessage (header + body array)
// =============================================================================

/**
 * Pack a complete MitchMessage (header + all body entries) into wire bytes.
 *
 * The header.count field must equal body.length.
 */
export function packMitchMessage(msg: MitchMessage): Uint8Array {
  const { header } = msg;
  const type  = headerMsgType(header);
  const count = header.count;

  let bodySize = 0;
  switch (type) {
    case MessageType.TICK:
      bodySize = count * MESSAGE_SIZES.TICK;
      break;
    case MessageType.TRADE:
      bodySize = count * MESSAGE_SIZES.TRADE;
      break;
    case MessageType.ORDER:
      bodySize = count * MESSAGE_SIZES.ORDER;
      break;
    case MessageType.INDEX:
      bodySize = count * MESSAGE_SIZES.INDEX;
      break;
    case MessageType.ORDER_BOOK:
      bodySize = count * MESSAGE_SIZES.ORDER_BOOK;
      break;
  }

  const totalSize = MESSAGE_SIZES.HEADER + bodySize;
  const buf = new Uint8Array(totalSize);

  // Header
  const hdv = packHeader(header);
  new Uint8Array(buf.buffer, 0, SIZE_HEADER).set(new Uint8Array(hdv.buffer));

  let off = MESSAGE_SIZES.HEADER;

  switch (type) {
    case MessageType.TICK:
      for (const tick of msg.body as Tick[]) {
        const packed = packTick(tick);
        buf.set(packed, off);
        off += 32;
      }
      break;

    case MessageType.TRADE:
      for (const trade of msg.body as Trade[]) {
        const packed = packTrade(trade);
        buf.set(packed, off);
        off += MESSAGE_SIZES.TRADE;
      }
      break;

    case MessageType.ORDER:
      for (const order of msg.body as Order[]) {
        const packed = packOrder(order);
        buf.set(packed, off);
        off += MESSAGE_SIZES.ORDER;
      }
      break;

    case MessageType.INDEX:
      for (const index of msg.body as Index[]) {
        const packed = packIndex(index);
        buf.set(packed, off);
        off += MESSAGE_SIZES.INDEX;
      }
      break;

    case MessageType.ORDER_BOOK:
      for (const ob of msg.body as OptimizedOrderBook[]) {
        const packed = packOrderBook(ob);
        buf.set(packed, off);
        off += MESSAGE_SIZES.ORDER_BOOK;
      }
      break;
  }

  return buf;
}

/**
 * Unpack wire bytes into a MitchMessage.
 * Reads the header first, then decodes count body entries.
 *
 * @throws RangeError if bytes is too short for the declared count.
 */
export function unpackMitchMessage(bytes: Uint8Array): MitchMessage {
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const header = unpackHeader(dv);
  const type  = headerMsgType(header);
  const count = header.count;
  const body: unknown[] = [];
  let off = MESSAGE_SIZES.HEADER;

  switch (type) {
    case MessageType.TICK:
      for (let i = 0; i < count; i++) {
        body.push(unpackTick(bytes.subarray(off, off + 32)));
        off += 32;
      }
      return { header, body: body as Tick[] };

    case MessageType.TRADE:
      for (let i = 0; i < count; i++) {
        body.push(unpackTrade(bytes.subarray(off, off + MESSAGE_SIZES.TRADE)));
        off += MESSAGE_SIZES.TRADE;
      }
      return { header, body: body as Trade[] };

    case MessageType.ORDER:
      for (let i = 0; i < count; i++) {
        body.push(unpackOrder(bytes.subarray(off, off + MESSAGE_SIZES.ORDER)));
        off += MESSAGE_SIZES.ORDER;
      }
      return { header, body: body as Order[] };

    case MessageType.INDEX:
      for (let i = 0; i < count; i++) {
        body.push(unpackIndex(bytes.subarray(off, off + MESSAGE_SIZES.INDEX)));
        off += MESSAGE_SIZES.INDEX;
      }
      return { header, body: body as Index[] };

    case MessageType.ORDER_BOOK:
      for (let i = 0; i < count; i++) {
        body.push(unpackOrderBook(bytes.subarray(off, off + MESSAGE_SIZES.ORDER_BOOK)));
        off += MESSAGE_SIZES.ORDER_BOOK;
      }
      return { header, body: body as OptimizedOrderBook[] };

    default:
      throw new RangeError(`Unknown MITCH message type: ${type}`);
  }
}
