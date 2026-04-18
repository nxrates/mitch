# Nasdaq ITCH 5.0 Complete Implementation Reference (TCP/IP Stream)

> **Note**: This summary is derived from the official Nasdaq ITCH 5.0 specifications and reference documentation, and serves as the foundational basis for the MITCH specification implementation.

## 1. TCP/IP Stream Protocol

ITCH messages are encapsulated in a SoupBinTCP stream. Each packet has a header followed by one or more ITCH messages.

```
┌────────────────┬───────────────────┬──────────────────┬────────────────────────┐
│ Session (2B)   │ Sequence (8B)     │ Count (2B)       │ Messages (Variable)    │
│ Big-endian     │ Big-endian        │ Big-endian       │ (Count * Message Length) │
└────────────────┴───────────────────┴──────────────────┴────────────────────────┘
```

- **Session**: Identifies the logical session.
- **Sequence**: Message sequence number.
- **Count**: Number of ITCH messages in the payload.

---

## 2. 🧠 Core Data Format

* **Byte Order**: Big-endian (network byte order) for all multi-byte fields.
* **Text Fields (`Alpha`)**: Left-justified, space-padded printable ASCII characters.
* **Timestamps**: 48-bit nanoseconds since midnight (00:00:00.000000000).
* **Price Formats**:
  - **`Price(4)`**: 4-byte unsigned integer, 4 implied decimal places.
  - **`Price(8)`**: 8-byte unsigned integer, 8 implied decimal places.
  - *Example `Price(4)`*: `1234500` -> `123.4500`.

---

## 3. 🗳️ Message Types & Sizes

| Type | Name                              | Size | Usage Priority |
| ---- | --------------------------------- | ---- | -------------- |
| `S`  | System Event                      | 12B  | Essential      |
| `R`  | Stock Directory                   | 39B  | Essential      |
| `H`  | Stock Trading Action              | 25B  | High           |
| `Y`  | Reg SHO Restriction               | 20B  | Medium         |
| `L`  | Market Participant Position       | 26B  | Medium         |
| `V`  | MWCB Decline Level                | 35B  | Low            |
| `W`  | MWCB Status                       | 12B  | Low            |
| `K`  | Quoting Period Update (IPO)       | 28B  | Medium         |
| `J`  | LULD Auction Collar               | 35B  | Medium         |
| `h`  | Operational Halt                  | 21B  | High           |
| `A`  | Add Order (No MPID)               | 36B  | **Critical**   |
| `F`  | Add Order (With MPID)             | 40B  | **Critical**   |
| `E`  | Order Executed                    | 31B  | **Critical**   |
| `C`  | Order Executed w/ Price           | 36B  | **Critical**   |
| `X`  | Order Cancel                      | 23B  | **Critical**   |
| `D`  | Order Delete                      | 19B  | **Critical**   |
| `U`  | Order Replace                     | 35B  | **Critical**   |
| `P`  | Trade (Non-Cross)                 | 44B  | **Critical**   |
| `Q`  | Cross Trade                       | 40B  | High           |
| `B`  | Broken Trade                      | 19B  | High           |
| `I`  | Net Order Imbalance (NOII)        | 50B  | Medium         |
| `N`  | Retail Price Improvement          | 20B  | Low            |

---

## 4. 📦 Essential Message Layouts

*Note: The `Stock` field in messages like Add Order (`A`/`F`) is only populated if `Stock Locate` is 0.*

### System Event (`S`) - 12 bytes

| Field        | Offset | Size | Type   | Values                    |
| ------------ | ------ | ---- | ------ | ------------------------- |
| Message Type | 0      | 1    | `S`    |                           |
| Stock Locate | 1      | 2    | UInt16 | 0 = Not Applicable        |
| Tracking No. | 3      | 2    | UInt16 |                           |
| Timestamp    | 5      | 6    | UInt48 | Nanoseconds since midnight|
| Event Code   | 11     | 1    | Alpha  | `O`: Start of Msgs, `S`: Start of Sys Hours, `Q`: Start of Mkt Hours, `M`: End of Mkt Hours, `E`: End of Sys Hours, `C`: End of Msgs, `A`: Trading Resumes |

### Stock Directory (`R`) - 39 bytes

| Field                | Offset | Size | Type   | Values               |
| -------------------- | ------ | ---- | ------ | -------------------- |
| Message Type         | 0      | 1    | `R`    |                      |
| Stock Locate         | 1      | 2    | UInt16 | Instrument reference |
| Tracking No.         | 3      | 2    | UInt16 |                      |
| Timestamp            | 5      | 6    | UInt48 |                      |
| Stock Symbol         | 11     | 8    | Alpha  | Right-padded w/spaces|
| Market Category      | 19     | 1    | Alpha  | `Q`: NASDAQ GS, `G`: NASDAQ GM, `S`: NASDAQ CM |
| Financial Status     | 20     | 1    | Alpha  | `D`: Deficient, `E`: Delinquent, `Q`: Bankrupt, `S`: Suspended, `G`: Deficient/Bankrupt, `H`: Deficient/Delinquent, `J`: Delinquent/Bankrupt, `K`: Deficient/Delinquent/Bankrupt, `C`: ETP Suspended, `N`: Normal, ` `: N/A |
| Round Lot Size       | 21     | 4    | UInt32 | Shares per round lot |
| Round Lots Only      | 25     | 1    | Alpha  | `Y`=Yes, `N`=No      |
| Issue Classification | 26     | 1    | Alpha  | `A`: ADS, `B`: Bond, `C`: Common, `F`: Depository Receipt, `I`: 144A, `L`: Ltd P'ship, `N`: Notes, `O`: Ordinary Share, `P`: Pfd, `Q`: Other, `R`: Right, `S`: Shrs Ben Int, `T`: Conv Debenture, `U`: Unit, `V`: Units Ben Int, `W`: Warrant |
| Issue Sub-Type       | 27     | 2    | Alpha  | See official ITCH spec for full list |
| Authenticity         | 29     | 1    | Alpha  | `P`: Live, `T`: Test |
| Short Sale Threshold | 30     | 1    | Alpha  | `Y`=Yes, `N`=No, ` `=NA |
| IPO Flag             | 31     | 1    | Alpha  | `Y`=Yes, `N`=No, ` `=NA |
| LULD Tier            | 32     | 1    | Alpha  | `1`=Tier 1, `2`=Tier 2, ` `=NA |
| ETP Flag             | 33     | 1    | Alpha  | `Y`=Yes, `N`=No, ` `=NA |
| ETP Leverage Factor  | 34     | 4    | UInt32 | Integer (e.g., 200 for 2x) |
| Inverse Flag         | 38     | 1    | Alpha  | `Y`=Yes, `N`=No      |

### Stock Trading Action (`H`) - 25 bytes

| Field         | Offset | Size | Type   | Values                    |
| ------------- | ------ | ---- | ------ | ------------------------- |
| Message Type  | 0      | 1    | `H`    |                           |
| Stock Locate  | 1      | 2    | UInt16 |                           |
| Tracking No.  | 3      | 2    | UInt16 |                           |
| Timestamp     | 5      | 6    | UInt48 |                           |
| Stock         | 11     | 8    | Alpha  |                           |
| Trading State | 19     | 1    | Alpha  | `H`=Halted, `P`=Paused, `Q`=Quotation only, `T`=Trading |
| Reserved      | 20     | 1    | Alpha  | Always ` ` (space)        |
| Reason        | 21     | 4    | Alpha  | See trading action codes  |

### Add Order – No MPID (`A`) - 36 bytes

| Field         | Offset | Size | Type     | Values              |
| ------------- | ------ | ---- | -------- | ------------------- |
| Message Type  | 0      | 1    | `A`      |                     |
| Stock Locate  | 1      | 2    | UInt16   |                     |
| Tracking No.  | 3      | 2    | UInt16   |                     |
| Timestamp     | 5      | 6    | UInt48   |                     |
| Order Ref No. | 11     | 8    | UInt64   | Unique order ID     |
| Buy/Sell      | 19     | 1    | Alpha    | `B`=Buy, `S`=Sell   |
| Shares        | 20     | 4    | UInt32   |                     |
| Stock         | 24     | 8    | Alpha    |                     |
| Price         | 32     | 4    | Price(4) | In 1/10000 dollars  |

### Add Order – MPID (`F`) - 40 bytes

Same as `A` plus:

| Field       | Offset | Size | Type  |
| ----------- | ------ | ---- | ----- |
| Attribution | 36     | 4    | Alpha |

### Order Executed (`E`) - 31 bytes

| Field           | Offset | Size | Type   |
| --------------- | ------ | ---- | ------ |
| Message Type    | 0      | 1    | `E`    |
| Stock Locate    | 1      | 2    | UInt16 |
| Tracking No.    | 3      | 2    | UInt16 |
| Timestamp       | 5      | 6    | UInt48 |
| Order Ref No.   | 11     | 8    | UInt64 |
| Executed Shares | 19     | 4    | UInt32 |
| Match Number    | 23     | 8    | UInt64 |

### Order Executed w/ Price (`C`) - 36 bytes

Same as `E` plus:

| Field     | Offset | Size | Type     | Values              |
| --------- | ------ | ---- | -------- | ------------------- |
| Printable | 31     | 1    | Alpha    | `Y`=Yes, `N`=No     |
| Price     | 32     | 4    | Price(4) |                     |

### Order Cancel (`X`) - 23 bytes

| Field            | Offset | Size | Type   |
| ---------------- | ------ | ---- | ------ |
| Message Type     | 0      | 1    | `X`    |
| Stock Locate     | 1      | 2    | UInt16 |
| Tracking No.     | 3      | 2    | UInt16 |
| Timestamp        | 5      | 6    | UInt48 |
| Order Ref No.    | 11     | 8    | UInt64 |
| Cancelled Shares | 19     | 4    | UInt32 |

### Order Delete (`D`) - 19 bytes

| Field         | Offset | Size | Type   |
| ------------- | ------ | ---- | ------ |
| Message Type  | 0      | 1    | `D`    |
| Stock Locate  | 1      | 2    | UInt16 |
| Tracking No.  | 3      | 2    | UInt16 |
| Timestamp     | 5      | 6    | UInt48 |
| Order Ref No. | 11     | 8    | UInt64 |

### Order Replace (`U`) - 35 bytes

| Field              | Offset | Size | Type     |
| ------------------ | ------ | ---- | -------- |
| Message Type       | 0      | 1    | `U`      |
| Stock Locate       | 1      | 2    | UInt16   |
| Tracking No.       | 3      | 2    | UInt16   |
| Timestamp          | 5      | 6    | UInt48   |
| Orig Order Ref No. | 11     | 8    | UInt64   |
| New Order Ref No.  | 19     | 8    | UInt64   |
| Shares             | 27     | 4    | UInt32   |
| Price              | 31     | 4    | Price(4) |

### Trade Message (`P`) - 44 bytes

| Field         | Offset | Size | Type     |
| ------------- | ------ | ---- | -------- |
| Message Type  | 0      | 1    | `P`      |
| Stock Locate  | 1      | 2    | UInt16   |
| Tracking No.  | 3      | 2    | UInt16   |
| Timestamp     | 5      | 6    | UInt48   |
| Order Ref No. | 11     | 8    | UInt64   |
| Buy/Sell      | 19     | 1    | Alpha    |
| Shares        | 20     | 4    | UInt32   |
| Stock         | 24     | 8    | Alpha    |
| Price         | 32     | 4    | Price(4) |
| Match No.     | 36     | 8    | UInt64   |

### Cross Trade (`Q`) - 40 bytes

| Field        | Offset | Size | Type     | Values               |
| ------------ | ------ | ---- | -------- | -------------------- |
| Message Type | 0      | 1    | `Q`      |                      |
| Stock Locate | 1      | 2    | UInt16   |                      |
| Tracking No. | 3      | 2    | UInt16   |                      |
| Timestamp    | 5      | 6    | UInt48   |                      |
| Shares       | 11     | 8    | UInt64   |                      |
| Stock        | 19     | 8    | Alpha    |                      |
| Price        | 27     | 4    | Price(4) |                      |
| Match No.    | 31     | 8    | UInt64   |                      |
| Cross Type   | 39     | 1    | Alpha    | `O`: Opening, `C`: Closing, `H`: IPO/Halt, `I`: Intraday |

### Broken Trade (`B`) - 19 bytes

| Field        | Offset | Size | Type   |
| ------------ | ------ | ---- | ------ |
| Message Type | 0      | 1    | `B`    |
| Stock Locate | 1      | 2    | UInt16 |
| Tracking No. | 3      | 2    | UInt16 |
| Timestamp    | 5      | 6    | UInt48 |
| Match No.    | 11     | 8    | UInt64 |

### Net Order Imbalance Indicator (`I`) - 50 bytes

| Field                | Offset | Size | Type     | Description/Values |
| -------------------- | ------ | ---- | -------- | ------------------ |
| Message Type         | 0      | 1    | `I`      |                    |
| Stock Locate         | 1      | 2    | UInt16   |                    |
| Tracking No.         | 3      | 2    | UInt16   |                    |
| Timestamp            | 5      | 6    | UInt48   |                    |
| Paired Shares        | 11     | 8    | UInt64   | Paired in cross    |
| Imbalance Shares     | 19     | 8    | UInt64   | Unpaired in cross  |
| Imbalance Direction  | 27     | 1    | Alpha    | `B`: Buy, `S`: Sell, `N`: No Imbalance, `O`: Insufficient orders to calculate |
| Stock                | 28     | 8    | Alpha    |                    |
| Far Price            | 36     | 4    | Price(4) | Clearing price if all orders crossed |
| Near Price           | 40     | 4    | Price(4) | Clearing price if only market orders crossed |
| Current Ref Price    | 44     | 4    | Price(4) | Current indicative reference price |
| Cross Type           | 48     | 1    | Alpha    | `O`: Opening, `C`: Closing, `H`: IPO/Halt, `I`: Intraday |
| Price Var Indicator  | 49     | 1    | Alpha    | Price variance from last reference price (` `, `<`, `>`, `N`: No change) |

---

## 5. ⚡ TCP Implementation Notes

### Message Framing
```
┌─────────────┬─────────────────────────────┐
│ Length (2B) │ ITCH Message (Variable)     │
│ Big-endian  │ As per layouts above        │
└─────────────┴─────────────────────────────┘
```

### Parsing Strategy
1. **Read 2-byte length** (big-endian)
2. **Read length bytes** into buffer
3. **Dispatch by message_type[0]** to appropriate decoder
4. **Use fixed-size structs** for each message type

### Performance Tips
- **Pre-allocate symbol table** from `R` messages
- **Use Stock Locate as array index** for O(1) symbol lookup
- **Buffer pool reuse** for zero-allocation parsing
- **SIMD operations** for multi-message processing

### Critical Order Book Reconstruction
```
OrderBook maintenance requires: A, F, E, C, X, D, U
Price formation requires: P, Q
System state requires: S, H, R, h, Y, L
Imbalance analysis requires: I
```

---

## 6. 🔧 Implementation Examples

### Price Conversion
```c
// Convert Price(4) to double
double price_to_double(uint32_t price_int) {
    return (double)price_int / 10000.0;
}

// Convert double to Price(4)
uint32_t double_to_price(double price) {
    return (uint32_t)(price * 10000.0 + 0.5);
}
```

### Timestamp Handling
```c
// Convert UInt48 timestamp to nanoseconds
uint64_t read_timestamp(const uint8_t* buf) {
    return ((uint64_t)buf[0] << 40) | ((uint64_t)buf[1] << 32) |
           ((uint64_t)buf[2] << 24) | ((uint64_t)buf[3] << 16) |
           ((uint64_t)buf[4] << 8)  | (uint64_t)buf[5];
}
```

### Message Dispatcher
```c
typedef struct {
    uint8_t type;
    uint16_t stock_locate;
    uint16_t tracking_number;
    uint64_t timestamp;
} itch_header_t;

void process_message(const uint8_t* data, size_t len) {
    switch (data[0]) {
        case 'A': process_add_order_no_mpid(data); break;
        case 'F': process_add_order_mpid(data); break;
        case 'E': process_order_executed(data); break;
        case 'C': process_order_executed_with_price(data); break;
        case 'X': process_order_cancel(data); break;
        case 'D': process_order_delete(data); break;
        case 'U': process_order_replace(data); break;
        case 'P': process_trade(data); break;
        case 'Q': process_cross_trade(data); break;
        case 'B': process_broken_trade(data); break;
        case 'R': process_stock_directory(data); break;
        case 'S': process_system_event(data); break;
        case 'H': process_trading_action(data); break;
        default: /* handle unknown message types */;
    }
}
```

---

This reference provides complete implementation coverage for production-grade ITCH v5.0 adapters across multiple programming language, optimized for TCP transport with zero-copy parsing capabilities.
