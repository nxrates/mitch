//+------------------------------------------------------------------+
//| Concise MITCH Protocol Example in MQL4                          |
//| Demonstrates usage of mitch.mq4 reference implementation        |
//| for sending/receiving messages over TCP and file I/O            |
//+------------------------------------------------------------------+

#include "../mitch.mq4"

//+------------------------------------------------------------------+
//| Example: Create and pack Index message                          |
//+------------------------------------------------------------------+
void CreateIndexMessage()
{
    // Create message using reference implementation
    MitchHeader header;
    header.messageType = MessageTypeIndex;
    header.timestamp = 1234567890123456;
    header.count = 1;

    Index index;
    index.tickerId = 0x03006F301CD00000; // EUR/USD
    index.mid = 1.08750;
    index.vbid = 500000;
    index.vask = 600000;
    index.mspread = 150;
    index.bbido = -50;
    index.basko = 100;
    index.wbido = -100;
    index.wasko = 200;
    index.vforce = 7500;
    index.lforce = 8500;
    index.tforce = 250;
    index.mforce = -150;
    index.confidence = 95;
    index.rejected = 1;
    index.accepted = 10;

    // Pack using reference implementation
    uchar buffer[72]; // 8 bytes header + 64 bytes index
    PackHeader(header, buffer);
    PackIndex(index, buffer, 8);

    Print("Created Index message (", ArraySize(buffer), " bytes)");
    Print("Ticker ID: 0x", IntegerToHexString(index.tickerId));
    Print("Mid price: ", DoubleToString(index.mid, 5));
    Print("confidence: ", index.confidence);
}

//+------------------------------------------------------------------+
//| Example: Create and pack Trade message                          |
//+------------------------------------------------------------------+
void CreateTradeMessage()
{
    // Create trade message
    MitchHeader header;
    header.messageType = MessageTypeTrade;
    header.timestamp = 1234567890123456;
    header.count = 1;

    Trade trade;
    trade.tickerId = 0x03006F301CD00000; // EUR/USD
    trade.price = 1.08750;
    trade.quantity = 100000;
    trade.tradeId = 12345;
    trade.side = 0; // Buy

    // Pack using reference implementation
    uchar buffer[40]; // 8 bytes header + 32 bytes trade
    PackHeader(header, buffer);
    PackTrade(trade, buffer, 8);

    Print("Created Trade message (", ArraySize(buffer), " bytes)");
    Print("Ticker ID: 0x", IntegerToHexString(trade.tickerId));
    Print("Price: ", DoubleToString(trade.price, 5));
    Print("Quantity: ", trade.quantity);
    Print("Side: ", (trade.side == 0) ? "Buy" : "Sell");
}

//+------------------------------------------------------------------+
//| Example: Unpack received message                                |
//+------------------------------------------------------------------+
void UnpackReceivedMessage()
{
    // Simulate received message bytes
    uchar receivedBuffer[72];

    // First create a message to simulate receiving it
    MitchHeader header;
    header.messageType = MessageTypeIndex;
    header.timestamp = 1234567890123456;
    header.count = 1;

    Index index;
    index.tickerId = 0x03006F301CD00000;
    index.mid = 1.08750;
    index.vbid = 500000;
    index.vask = 600000;
    index.mspread = 150;
    index.bbido = -50;
    index.basko = 100;
    index.wbido = -100;
    index.wasko = 200;
    index.vforce = 7500;
    index.lforce = 8500;
    index.tforce = 250;
    index.mforce = -150;
    index.confidence = 95;
    index.rejected = 1;
    index.accepted = 10;

    PackHeader(header, receivedBuffer);
    PackIndex(index, receivedBuffer, 8);

    // Now unpack the "received" message
    MitchHeader unpackedHeader;
    UnpackHeader(receivedBuffer, unpackedHeader);

    Print("Unpacked message type: ", CharToString(unpackedHeader.messageType));
    Print("Timestamp: ", unpackedHeader.timestamp);
    Print("Count: ", unpackedHeader.count);

    if(unpackedHeader.messageType == MessageTypeIndex)
    {
        Index unpackedIndex;
        UnpackIndex(receivedBuffer, unpackedIndex, 8);

        Print("Unpacked Index:");
        Print("  Ticker ID: 0x", IntegerToHexString(unpackedIndex.tickerId));
        Print("  Mid: ", DoubleToString(unpackedIndex.mid, 5));
        Print("  vbid: ", unpackedIndex.vbid);
        Print("  vask: ", unpackedIndex.vask);
        Print("  confidence: ", unpackedIndex.confidence);
    }
}

//+------------------------------------------------------------------+
//| Example: Write message to file                                  |
//+------------------------------------------------------------------+
void WriteMessageToFile()
{
    // Create trade message
    MitchHeader header;
    header.messageType = MessageTypeTrade;
    header.timestamp = GetTickCount64();
    header.count = 2; // Multiple trades

    Trade trades[2];

    // First trade
    trades[0].tickerId = 0x03006F301CD00000; // EUR/USD
    trades[0].price = 1.08750;
    trades[0].quantity = 100000;
    trades[0].tradeId = 12345;
    trades[0].side = 0; // Buy

    // Second trade
    trades[1].tickerId = 0x03006F301CD00000; // EUR/USD
    trades[1].price = 1.08755;
    trades[1].quantity = 50000;
    trades[1].tradeId = 12346;
    trades[1].side = 1; // Sell

    // Pack message
    uchar buffer[72]; // 8 bytes header + 2 * 32 bytes trades
    PackHeader(header, buffer);

    for(int i = 0; i < 2; i++)
    {
        PackTrade(trades[i], buffer, 8 + i * 32);
    }

    // Write to file
    int fileHandle = FileOpen("mitch_trades.bin", FILE_WRITE | FILE_BIN);
    if(fileHandle != INVALID_HANDLE)
    {
        FileWriteArray(fileHandle, buffer, 0, ArraySize(buffer));
        FileClose(fileHandle);
        Print("Wrote ", ArraySize(buffer), " bytes to mitch_trades.bin");
    }
    else
    {
        Print("Failed to open file for writing");
    }
}

//+------------------------------------------------------------------+
//| Example: Read message from file                                 |
//+------------------------------------------------------------------+
void ReadMessageFromFile()
{
    int fileHandle = FileOpen("mitch_trades.bin", FILE_READ | FILE_BIN);
    if(fileHandle != INVALID_HANDLE)
    {
        uchar buffer[72];
        int bytesRead = FileReadArray(fileHandle, buffer, 0, ArraySize(buffer));
        FileClose(fileHandle);

        if(bytesRead > 0)
        {
            Print("Read ", bytesRead, " bytes from mitch_trades.bin");

            // Unpack header
            MitchHeader header;
            UnpackHeader(buffer, header);

            Print("Message type: ", CharToString(header.messageType));
            Print("Count: ", header.count);

            if(header.messageType == MessageTypeTrade)
            {
                for(int i = 0; i < header.count; i++)
                {
                    Trade trade;
                    UnpackTrade(buffer, trade, 8 + i * 32);

                    Print("Trade ", i + 1, ":");
                    Print("  Price: ", DoubleToString(trade.price, 5));
                    Print("  Quantity: ", trade.quantity);
                    Print("  Side: ", (trade.side == 0) ? "Buy" : "Sell");
                }
            }
        }
    }
    else
    {
        Print("Failed to open file for reading");
    }
}

//+------------------------------------------------------------------+
//| Script program start function                                   |
//+------------------------------------------------------------------+
void OnStart()
{
    Print("MITCH Protocol MQL4 Examples");
    Print("============================");

    Print("1. Creating Index message");
    CreateIndexMessage();
    Print("");

    Print("2. Creating Trade message");
    CreateTradeMessage();
    Print("");

    Print("3. Unpacking received message");
    UnpackReceivedMessage();
    Print("");

    Print("4. Writing message to file");
    WriteMessageToFile();
    Print("");

    Print("5. Reading message from file");
    ReadMessageFromFile();
    Print("");

    Print("Examples completed!");
}
