/**
 * Mitch.cs - MITCH Protocol .NET 6+ Implementation
 *
 * Uses StructLayout(Sequential, Pack=1) for zero-copy wire compatibility.
 * BinaryPrimitives for portable endianness. Unsafe generic pack/unpack for
 * unmanaged structs. Extension methods for helpers.
 *
 * Binary layout reference (all little-endian):
 *   Header    16 bytes
 *   Tick      32 bytes
 *   Trade     24 bytes
 *   Order     32 bytes
 *   Index     40 bytes
 *   Bin        8 bytes
 *   Bar       96 bytes
 *   OrderBook  2072 bytes
 */

using System;
using System.Buffers.Binary;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Mitch;

// =============================================================================
// ENUMS
// =============================================================================

/// <summary>MITCH message type codes (ASCII).</summary>
public enum MessageType : byte
{
    /// <summary>Tick/quote snapshot - 's' (115)</summary>
    Tick      = (byte)'s',
    /// <summary>Trade execution - 't' (116)</summary>
    Trade     = (byte)'t',
    /// <summary>Order lifecycle - 'o' (111)</summary>
    Order     = (byte)'o',
    /// <summary>Index aggregated - 'i' (105)</summary>
    Index     = (byte)'i',
    /// <summary>Order book snapshot - 'b' (98)</summary>
    OrderBook = (byte)'b',
    /// <summary>Bar/candle - 'k' (107)</summary>
    Bar       = (byte)'k',
}

/// <summary>Order side.</summary>
public enum OrderSide : byte
{
    Buy  = 0,
    Sell = 1,
}

/// <summary>Order type (stored in bits [7:1] of TypeAndSide).</summary>
public enum OrderType : byte
{
    Market = 0,
    Limit  = 1,
    Stop   = 2,
    Cancel = 3,
}

/// <summary>Asset class (4-bit value in ticker ID).</summary>
public enum AssetClass : byte
{
    EQ  = 0,   // Equities
    CB  = 1,   // Corporate bonds
    SD  = 2,   // Sovereign debt
    FX  = 3,   // Forex
    CM  = 4,   // Commodities
    RE  = 5,   // Real estate
    CR  = 6,   // Crypto assets
    PM  = 7,   // Private markets
    CL  = 8,   // Collectibles
    IN  = 9,   // Infrastructure
    IP  = 10,  // Indices/products
    SP  = 11,  // Structured products
    CE  = 12,  // Cash equivalents
    LR  = 13,  // Loans/receivables
}

/// <summary>Instrument type (4-bit value, ticker ID bits [63:60]).</summary>
public enum InstrType : byte
{
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
}

/// <summary>Order book bin aggregation method.</summary>
public enum BinAgg : byte
{
    Trilinear   = 0,
    Lingaussian = 1,
    Bilingeo    = 2,
    Lingeoflat  = 3,
}

/// <summary>Compact numeric wire codes for the header type_provider field.</summary>
public enum WireCode : byte
{
    Trade     = 1,
    Order     = 2,
    Tick      = 3,
    Index     = 4,
    OrderBook = 5,
    Bar       = 6,
}

/// <summary>Wire code mapping utilities.</summary>
public static class WireCodeMap
{
    /// <summary>Map a wire code to its ASCII message type byte.</summary>
    public static byte WireToAscii(WireCode code) => code switch
    {
        WireCode.Trade     => (byte)'t',
        WireCode.Order     => (byte)'o',
        WireCode.Tick      => (byte)'s',
        WireCode.Index     => (byte)'i',
        WireCode.OrderBook => (byte)'b',
        WireCode.Bar       => (byte)'k',
        _ => throw new ArgumentOutOfRangeException(nameof(code), $"Unknown wire code: {code}"),
    };

    /// <summary>Map an ASCII message type byte to its wire code.</summary>
    public static WireCode AsciiToWire(byte ascii) => ascii switch
    {
        (byte)'t' => WireCode.Trade,
        (byte)'o' => WireCode.Order,
        (byte)'s' => WireCode.Tick,
        (byte)'i' => WireCode.Index,
        (byte)'b' => WireCode.OrderBook,
        (byte)'k' => WireCode.Bar,
        _ => throw new ArgumentOutOfRangeException(nameof(ascii), $"Unknown ASCII message type: {(char)ascii}"),
    };
}

// =============================================================================
// SIZE CONSTANTS
// =============================================================================

/// <summary>Wire sizes in bytes for each MITCH message body type.</summary>
public static class MitchSize
{
    public const int Header    =   16;
    public const int Tick      =   32;
    public const int Trade     =   24;
    public const int Order     =   32;
    public const int Index     =   40;
    public const int Bin       =    8;
    public const int Bar       =   96;
    public const int OrderBook = 2072;
}

// =============================================================================
// KNOWN MARKET PROVIDER IDS
// =============================================================================

/// <summary>Known MITCH market provider IDs.</summary>
public static class Provider
{
    public const ushort Binance   = 101;
    public const ushort BingX     = 111;
    public const ushort Bitget    = 141;
    public const ushort BitMart   = 161;
    public const ushort Bitstamp  = 181;
    public const ushort Bitunix   = 191;
    public const ushort Bullish   = 251;
    public const ushort Bybit     = 261;
    public const ushort Coinbase  = 341;
    public const ushort CryptoCom = 391;
    public const ushort Gate      = 561;
    public const ushort GroveX    = 583;
    public const ushort HTX       = 641;
    public const ushort Kraken    = 721;
    public const ushort KuCoin    = 741;
    public const ushort LBank     = 761;
    public const ushort MEXC      = 821;
    public const ushort OKX       = 911;
    public const ushort Toobit    = 1181;
    public const ushort Upbit     = 1251;
    public const ushort WhiteBIT  = 1281;
    public const ushort XT        = 1301;
}

// =============================================================================
// WIRE STRUCTURES
// =============================================================================

/// <summary>
/// 16-byte MITCH message header.
/// <para>Wire layout:</para>
/// <code>
///   [0..1]   TypeProvider : u16 LE - bits[3:0]=wire_code, bits[15:4]=provider_id
///   [2..7]   TimestampTicks : u48 LE - 16µs ticks since 2010-01-01T00:00:00Z
///   [8]      Count       : u8
///   [9]      Flags       : u8
///   [10..11] Sequence    : u16 LE
///   [12..15] Reserved    : 4 bytes
/// </code>
/// </summary>
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public unsafe struct Header
{
    /// <summary>Packed type+provider: bits[3:0]=wire_code, bits[15:4]=provider_id.</summary>
    public ushort TypeProvider;
    /// <summary>16µs ticks since 2010-01-01T00:00:00Z, stored as 6-byte LE u48.</summary>
    public fixed byte TimestampBytes[6];
    /// <summary>Number of body entries (1-255).</summary>
    public byte Count;
    /// <summary>Flags byte.</summary>
    public byte Flags;
    /// <summary>Sequence number.</summary>
    public ushort Sequence;
    /// <summary>Reserved (4 bytes).</summary>
    public fixed byte Reserved[4];

    /// <summary>Get/set TimestampTicks as a ulong (u48 range).</summary>
    public ulong TimestampTicks
    {
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        get
        {
            fixed (byte* p = TimestampBytes)
            {
                return MitchUtil.ReadU48(p);
            }
        }
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        set
        {
            fixed (byte* p = TimestampBytes)
            {
                MitchUtil.WriteU48(p, value);
            }
        }
    }

    /// <summary>Extract the ASCII message type byte from TypeProvider (via wire code).</summary>
    public byte MsgType => WireCodeMap.WireToAscii((WireCode)(TypeProvider & 0x0F));

    /// <summary>Extract the provider ID from TypeProvider (bits [15:4]).</summary>
    public ushort ProviderId => (ushort)((TypeProvider >> 4) & 0x0FFF);

    /// <summary>
    /// Create a Header from an ASCII message type, provider ID, and other fields.
    /// </summary>
    /// <param name="asciiType">ASCII message type byte (e.g. 's', 't', 'o', etc.)</param>
    /// <param name="providerId">Market provider ID (0-4095)</param>
    /// <param name="ticks">Timestamp in 16µs ticks</param>
    /// <param name="count">Body entry count (1-255)</param>
    /// <param name="flags">Flags byte</param>
    /// <param name="sequence">Sequence number (u16)</param>
    /// <returns>New Header</returns>
    public static Header Create(byte asciiType, ushort providerId,
                                 ulong ticks, byte count,
                                 byte flags = 0, ushort sequence = 0)
    {
        var wireCode = (byte)WireCodeMap.AsciiToWire(asciiType);
        var h = new Header
        {
            TypeProvider = (ushort)(((providerId & 0x0FFF) << 4) | (wireCode & 0x0F)),
            Count        = count,
            Flags        = flags,
            Sequence     = sequence,
        };
        h.TimestampTicks = ticks;
        return h;
    }
}

/// <summary>
/// 32-byte tick/quote snapshot.
/// <para>Wire layout:</para>
/// <code>
///   [0..7]   TickerId  : u64 LE
///   [8..15]  BidPrice  : f64 LE
///   [16..23] AskPrice  : f64 LE
///   [24..27] BidVolume : u32 LE
///   [28..31] AskVolume : u32 LE
/// </code>
/// </summary>
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct Tick
{
    public ulong  TickerId;
    public double BidPrice;
    public double AskPrice;
    public uint   BidVolume;
    public uint   AskVolume;

    /// <summary>Mid price: (BidPrice + AskPrice) / 2.</summary>
    public double Mid => (BidPrice + AskPrice) * 0.5;

    /// <summary>Bid-ask spread: AskPrice - BidPrice.</summary>
    public double Spread => AskPrice - BidPrice;
}

/// <summary>
/// 24-byte trade execution record.
/// <para>Wire layout:</para>
/// <code>
///   [0..7]   TickerId : u64 LE
///   [8..15]  Price    : f64 LE
///   [16..19] Volume   : u32 LE
///   [20..22] TradeId  : u24 LE (3 bytes, max 16_777_215)
///   [23]     Side     : u8  (0=Buy, 1=Sell)
/// </code>
/// </summary>
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public unsafe struct Trade
{
    public ulong     TickerId;
    public double    Price;
    public uint      Volume;
    /// <summary>Trade ID stored as 3-byte LE u24 (max 16_777_215).</summary>
    public fixed byte TradeIdBytes[3];
    public OrderSide Side;

    /// <summary>Get/set TradeId as a uint (u24 range).</summary>
    public uint TradeId
    {
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        get
        {
            fixed (byte* p = TradeIdBytes)
            {
                return (uint)p[0] | ((uint)p[1] << 8) | ((uint)p[2] << 16);
            }
        }
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        set
        {
            fixed (byte* p = TradeIdBytes)
            {
                p[0] = (byte)value;
                p[1] = (byte)(value >> 8);
                p[2] = (byte)(value >> 16);
            }
        }
    }
}

/// <summary>
/// 32-byte order lifecycle event.
/// <para>Wire layout:</para>
/// <code>
///   [0..7]   TickerId     : u64 LE
///   [8..11]  OrderId      : u32 LE
///   [12..19] Price        : f64 LE
///   [20..23] Quantity     : u32 LE
///   [24]     TypeAndSide  : u8  bits[7:1]=order_type, bit[0]=side
///   [25..30] Expiry       : u48 LE ms since epoch
///   [31]     padding      : u8
/// </code>
/// </summary>
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public unsafe struct Order
{
    public ulong TickerId;
    public uint  OrderId;
    public double Price;
    public uint  Quantity;
    /// <summary>bits[7:1] = OrderType, bit[0] = OrderSide.</summary>
    public byte  TypeAndSide;
    /// <summary>Expiry stored as 6-byte LE u48 (milliseconds since epoch).</summary>
    public fixed byte ExpiryBytes[6];
    public byte  _Pad;

    /// <summary>Build TypeAndSide byte from explicit type and side.</summary>
    public static byte MakeTypeAndSide(OrderType t, OrderSide s)
        => (byte)(((byte)t << 1) | ((byte)s & 0x01));

    /// <summary>Extract OrderSide from TypeAndSide.</summary>
    public OrderSide Side => (OrderSide)(TypeAndSide & 0x01);

    /// <summary>Extract OrderType from TypeAndSide.</summary>
    public OrderType Type => (OrderType)((TypeAndSide >> 1) & 0x7F);

    /// <summary>Get/set expiry as ulong (milliseconds since epoch, u48 range).</summary>
    public ulong ExpiryMs
    {
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        get
        {
            fixed (byte* p = ExpiryBytes)
            {
                return MitchUtil.ReadU48(p);
            }
        }
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        set
        {
            fixed (byte* p = ExpiryBytes)
            {
                MitchUtil.WriteU48(p, value);
            }
        }
    }
}

/// <summary>
/// 40-byte aggregated index snapshot.
/// <para>Wire layout:</para>
/// <code>
///   [0..7]   TickerId   : u64 LE
///   [8..15]  Bid        : f64 LE
///   [16..23] Ask        : f64 LE
///   [24..27] VBid       : u32 LE
///   [28..31] VAsk       : u32 LE
///   [32..33] CI         : u16 LE  (confidence interval, micro basis points)
///   [34..35] TickCount  : u16 LE
///   [36]     Confidence : u8
///   [37]     Accepted   : u8
///   [38]     Rejected   : u8
///   [39]     padding    : u8
/// </code>
/// </summary>
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct Index
{
    public ulong   TickerId;
    public double  Bid;
    public double  Ask;
    public uint    VBid;
    public uint    VAsk;
    public ushort  CI;
    public ushort  TickCount;
    public byte    Confidence;
    public byte    Accepted;
    public byte    Rejected;
    public byte    _Pad;
}

/// <summary>
/// 8-byte order book price-level bin.
/// <para>Wire layout:</para>
/// <code>
///   [0..3] OrderCount : u32 LE
///   [4..7] Volume     : u32 LE
/// </code>
/// </summary>
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct Bin
{
    public uint OrderCount;
    public uint Volume;
}

/// <summary>
/// 2072-byte aggregated order book snapshot.
/// <para>Wire layout:</para>
/// <code>
///   [0..7]       TickerId      : u64 LE
///   [8..15]      MidPrice      : f64 LE
///   [16]         BinAggregator : u8
///   [17..23]     padding       : 7 bytes
///   [24..1047]   Bids          : [Bin; 128]
///   [1048..2071] Asks          : [Bin; 128]
/// </code>
/// </summary>
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public unsafe struct OrderBook
{
    public ulong   TickerId;
    public double  MidPrice;
    public BinAgg  BinAggregator;
    public fixed byte _Pad[7];
    // Fixed arrays of Bin (each 8 bytes) - accessed via helpers below.
    // Inline storage: 128 * 8 = 1024 bytes bids + 1024 bytes asks.
    public fixed byte BidsRaw[1024];
    public fixed byte AsksRaw[1024];

    /// <summary>Read bid bin at index i (0-127).</summary>
    public Bin GetBid(int i)
    {
        if ((uint)i >= 128) throw new ArgumentOutOfRangeException(nameof(i));
        Bin b;
        fixed (byte* p = BidsRaw)
        {
            b.OrderCount = BinaryPrimitives.ReadUInt32LittleEndian(new ReadOnlySpan<byte>(p + i * 8, 4));
            b.Volume     = BinaryPrimitives.ReadUInt32LittleEndian(new ReadOnlySpan<byte>(p + i * 8 + 4, 4));
        }
        return b;
    }

    /// <summary>Write bid bin at index i (0-127).</summary>
    public void SetBid(int i, Bin bin)
    {
        if ((uint)i >= 128) throw new ArgumentOutOfRangeException(nameof(i));
        fixed (byte* p = BidsRaw)
        {
            BinaryPrimitives.WriteUInt32LittleEndian(new Span<byte>(p + i * 8, 4),     bin.OrderCount);
            BinaryPrimitives.WriteUInt32LittleEndian(new Span<byte>(p + i * 8 + 4, 4), bin.Volume);
        }
    }

    /// <summary>Read ask bin at index i (0-127).</summary>
    public Bin GetAsk(int i)
    {
        if ((uint)i >= 128) throw new ArgumentOutOfRangeException(nameof(i));
        Bin b;
        fixed (byte* p = AsksRaw)
        {
            b.OrderCount = BinaryPrimitives.ReadUInt32LittleEndian(new ReadOnlySpan<byte>(p + i * 8, 4));
            b.Volume     = BinaryPrimitives.ReadUInt32LittleEndian(new ReadOnlySpan<byte>(p + i * 8 + 4, 4));
        }
        return b;
    }

    /// <summary>Write ask bin at index i (0-127).</summary>
    public void SetAsk(int i, Bin bin)
    {
        if ((uint)i >= 128) throw new ArgumentOutOfRangeException(nameof(i));
        fixed (byte* p = AsksRaw)
        {
            BinaryPrimitives.WriteUInt32LittleEndian(new Span<byte>(p + i * 8, 4),     bin.OrderCount);
            BinaryPrimitives.WriteUInt32LittleEndian(new Span<byte>(p + i * 8 + 4, 4), bin.Volume);
        }
    }
}

// =============================================================================
// INTERNAL UTILITY
// =============================================================================

internal static unsafe class MitchUtil
{
    /// <summary>Read a 6-byte LE u48 from p.</summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    internal static ulong ReadU48(byte* p)
        => (ulong)p[0]
         | ((ulong)p[1] << 8)
         | ((ulong)p[2] << 16)
         | ((ulong)p[3] << 24)
         | ((ulong)p[4] << 32)
         | ((ulong)p[5] << 40);

    /// <summary>Write a ulong as a 6-byte LE u48 at p.</summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    internal static void WriteU48(byte* p, ulong v)
    {
        p[0] = (byte) v;
        p[1] = (byte)(v >> 8);
        p[2] = (byte)(v >> 16);
        p[3] = (byte)(v >> 24);
        p[4] = (byte)(v >> 32);
        p[5] = (byte)(v >> 40);
    }
}

// =============================================================================
// GENERIC PACK / UNPACK
// Requires `unsafe` project setting or <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
// =============================================================================

/// <summary>
/// Generic zero-copy pack and unpack for any unmanaged struct with a known wire size.
/// </summary>
public static unsafe class MitchSerializer
{
    /// <summary>
    /// Pack any unmanaged struct T into a new byte array.
    /// The struct must have [StructLayout(Sequential, Pack=1)].
    /// </summary>
    /// <typeparam name="T">Unmanaged struct type.</typeparam>
    /// <param name="value">Value to pack.</param>
    /// <returns>Wire bytes (little-endian, host-byte-order passthrough on LE hosts).</returns>
    public static byte[] Pack<T>(T value) where T : unmanaged
    {
        int size  = sizeof(T);
        var bytes = new byte[size];
        fixed (byte* dst = bytes)
        {
            Unsafe.CopyBlockUnaligned(dst, &value, (uint)size);
        }
        return bytes;
    }

    /// <summary>
    /// Pack any unmanaged struct T into a caller-supplied Span.
    /// </summary>
    /// <typeparam name="T">Unmanaged struct type.</typeparam>
    /// <param name="value">Value to pack.</param>
    /// <param name="dst">Destination span; must be at least sizeof(T) bytes.</param>
    public static void PackInto<T>(T value, Span<byte> dst) where T : unmanaged
    {
        int size = sizeof(T);
        if (dst.Length < size)
            throw new ArgumentException($"Destination span is too small: need {size} bytes");
        fixed (byte* p = dst)
        {
            Unsafe.CopyBlockUnaligned(p, &value, (uint)size);
        }
    }

    /// <summary>
    /// Unpack a ReadOnlySpan of bytes into an unmanaged struct T.
    /// The struct must have [StructLayout(Sequential, Pack=1)].
    /// </summary>
    /// <typeparam name="T">Unmanaged struct type.</typeparam>
    /// <param name="data">Source bytes; must be at least sizeof(T) bytes.</param>
    /// <returns>Decoded struct.</returns>
    public static T Unpack<T>(ReadOnlySpan<byte> data) where T : unmanaged
    {
        int size = sizeof(T);
        if (data.Length < size)
            throw new ArgumentException($"Source span is too small: need {size} bytes");
        T result;
        fixed (byte* p = data)
        {
            Unsafe.CopyBlockUnaligned(&result, p, (uint)size);
        }
        return result;
    }
}

// =============================================================================
// TICKER ID ENCODE / DECODE
//
// Bit layout (64-bit):
//   [63:60] InstrumentType  (4 bits)
//   [59:56] BaseAssetClass  (4 bits)
//   [55:40] BaseAssetID     (16 bits)
//   [39:36] QuoteAssetClass (4 bits)
//   [35:20] QuoteAssetID    (16 bits)
//   [19:0]  SubType         (20 bits)
// =============================================================================

/// <summary>Decoded components of a ticker ID.</summary>
public readonly struct TickerComponents
{
    public readonly InstrType  InstrType;
    public readonly AssetClass BaseClass;
    public readonly ushort     BaseId;
    public readonly AssetClass QuoteClass;
    public readonly ushort     QuoteId;
    public readonly uint       SubType;

    public TickerComponents(InstrType instrType, AssetClass baseClass, ushort baseId,
                            AssetClass quoteClass, ushort quoteId, uint subType)
    {
        InstrType  = instrType;
        BaseClass  = baseClass;
        BaseId     = baseId;
        QuoteClass = quoteClass;
        QuoteId    = quoteId;
        SubType    = subType;
    }
}

/// <summary>Ticker ID encode and decode utilities.</summary>
public static class TickerId
{
    /// <summary>
    /// Encode a 64-bit ticker ID from its components.
    /// </summary>
    /// <param name="instrType">  Instrument type  (InstrType enum)</param>
    /// <param name="baseClass">  Base asset class (AssetClass enum)</param>
    /// <param name="baseId">     Base asset ID    (0-65535)</param>
    /// <param name="quoteClass"> Quote asset class(AssetClass enum)</param>
    /// <param name="quoteId">    Quote asset ID   (0-65535)</param>
    /// <param name="subType">    Sub-type         (0-1048575)</param>
    /// <returns>64-bit ticker ID</returns>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ulong Encode(InstrType instrType, AssetClass baseClass, ushort baseId,
                               AssetClass quoteClass, ushort quoteId, uint subType = 0)
        => ((ulong)(byte)instrType  << 60)
         | ((ulong)(byte)baseClass  << 56)
         | ((ulong)baseId           << 40)
         | ((ulong)(byte)quoteClass << 36)
         | ((ulong)quoteId          << 20)
         | (subType & 0xFFFFFUL);

    /// <summary>
    /// Decode a 64-bit ticker ID into its components.
    /// </summary>
    /// <param name="tickerId">Raw 64-bit ticker ID</param>
    /// <returns>Decoded TickerComponents</returns>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static TickerComponents Decode(ulong tickerId)
        => new TickerComponents(
            instrType:  (InstrType) ((tickerId >> 60) & 0x0F),
            baseClass:  (AssetClass)((tickerId >> 56) & 0x0F),
            baseId:     (ushort)    ((tickerId >> 40) & 0xFFFF),
            quoteClass: (AssetClass)((tickerId >> 36) & 0x0F),
            quoteId:    (ushort)    ((tickerId >> 20) & 0xFFFF),
            subType:    (uint)       (tickerId        & 0xFFFFF)
        );
}

// =============================================================================
// CHANNEL ID UTILITIES
//
// 32-bit layout: [market_provider:16][message_type:8][padding:8]
// =============================================================================

/// <summary>Channel ID utilities for pub/sub routing.</summary>
public static class ChannelId
{
    /// <summary>Generate a 32-bit channel ID.</summary>
    /// <param name="providerId">Market provider ID (0-65535)</param>
    /// <param name="msgType">MITCH message type</param>
    /// <returns>32-bit channel ID</returns>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static uint Generate(ushort providerId, MessageType msgType)
        => ((uint)providerId << 16) | ((uint)(byte)msgType << 8);

    /// <summary>Extract the market provider ID from a channel ID.</summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ushort GetProvider(uint channelId)
        => (ushort)(channelId >> 16);

    /// <summary>Extract the MessageType from a channel ID.</summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static MessageType GetMsgType(uint channelId)
        => (MessageType)(byte)(channelId >> 8);
}

// =============================================================================
// EXTENSION METHODS - helpers on the wire structs
// =============================================================================

/// <summary>Extension helpers for Trade.</summary>
public static class TradeExtensions
{
    /// <summary>Returns true if the trade is a buy.</summary>
    public static bool IsBuy(this Trade t)  => t.Side == OrderSide.Buy;
    /// <summary>Returns true if the trade is a sell.</summary>
    public static bool IsSell(this Trade t) => t.Side == OrderSide.Sell;
}

/// <summary>Extension helpers for Order.</summary>
public static class OrderExtensions
{
    /// <summary>Returns true if this is a buy order.</summary>
    public static bool IsBuy(this Order o)  => o.Side == OrderSide.Buy;
    /// <summary>Returns true if this is a sell order.</summary>
    public static bool IsSell(this Order o) => o.Side == OrderSide.Sell;
}

/// <summary>Extension helpers for Tick.</summary>
public static class TickExtensions
{
    /// <summary>Mid price: (BidPrice + AskPrice) / 2.</summary>
    public static double Mid(this Tick t)    => (t.BidPrice + t.AskPrice) * 0.5;
    /// <summary>Bid-ask spread: AskPrice - BidPrice.</summary>
    public static double Spread(this Tick t) => t.AskPrice - t.BidPrice;
}

/// <summary>Extension helpers for Index.</summary>
public static class IndexExtensions
{
    /// <summary>Returns true if Confidence >= minConfidence.</summary>
    public static bool IsConfident(this Index idx, byte minConfidence)
        => idx.Confidence >= minConfidence;
}

// =============================================================================
// COMPILE-TIME SIZE ASSERTIONS
// These are evaluated at class initialisation; violations throw TypeInitializationException.
// =============================================================================

/// <summary>Validates wire sizes at runtime startup.</summary>
internal static unsafe class MitchSizeCheck
{
    static MitchSizeCheck()
    {
        static void Assert(int actual, int expected, string name)
        {
            if (actual != expected)
                throw new InvalidOperationException(
                    $"MITCH struct size mismatch: {name} is {actual} bytes, expected {expected}");
        }
        Assert(sizeof(Header),    MitchSize.Header,    nameof(Header));
        Assert(sizeof(Tick),      MitchSize.Tick,      nameof(Tick));
        Assert(sizeof(Trade),     MitchSize.Trade,     nameof(Trade));
        Assert(sizeof(Order),     MitchSize.Order,     nameof(Order));
        Assert(sizeof(Index),     MitchSize.Index,     nameof(Index));
        Assert(sizeof(Bin),       MitchSize.Bin,       nameof(Bin));
        Assert(sizeof(OrderBook), MitchSize.OrderBook, nameof(OrderBook));
    }

    // Force the static constructor to run early.
    internal static void Verify() { /* triggers static ctor */ }
}
