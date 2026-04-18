/**
 * Concise MITCH Protocol Example in TypeScript
 * Demonstrates usage of mitch.ts reference implementation
 * for sending/receiving messages over TCP and WebSocket
 */

import * as net from 'net';
import { WebSocket } from 'ws';
import { MitchMessage, MitchHeader, Trade, Index, MessageType, packMitchMessage, unpackMitchMessage } from '../mitch';

// Example: Send Index message over TCP
async function sendIndexTcp(): Promise<void> {
    return new Promise((resolve, reject) => {
        const client = new net.Socket();

        client.connect(8080, '127.0.0.1', () => {
            // Create message using reference implementation
            const header: MitchHeader = {
                messageType: MessageType.INDEX,
                timestamp: 1234567890123456n,
                count: 1
            };

            const index: Index = {
                tickerId: 0x03006F301CD00000n, // EUR/USD
                mid: 1.08750,
                vbid: 500000,
                vask: 600000,
                mspread: 150,
                bbido: -50,
                basko: 100,
                wbido: -100,
                wasko: 200,
                vforce: 7500,
                lforce: 8500,
                tforce: 250,
                mforce: -150,
                confidence: 95,
                rejected: 1,
                accepted: 10
            };

            const message: MitchMessage = {
                header,
                body: [index]
            };

            // Serialize using reference implementation
            const bytes = packMitchMessage(message);
            client.write(bytes);

            console.log(`Sent Index message successfully (${bytes.length} bytes)`);
            client.destroy();
            resolve();
        });

        client.on('error', (err) => {
            console.error('TCP connection error:', err);
            reject(err);
        });
    });
}

// Example: Receive messages over TCP
async function receiveTcp(): Promise<void> {
    return new Promise((resolve, reject) => {
        const client = new net.Socket();

        client.connect(8080, '127.0.0.1', () => {
            console.log('Connected to TCP server, waiting for data...');
        });

        client.on('data', (data) => {
            try {
                // Deserialize using reference implementation
                const message = unpackMitchMessage(data);

                console.log(`Received message type: ${String.fromCharCode(message.header.messageType)}, count: ${message.header.count}`);

                if (message.header.messageType === MessageType.INDEX) {
                    const indices = message.body as Index[];
                    indices.forEach((index, i) => {
                        console.log(`  Index ${i}: Ticker 0x${index.tickerId.toString(16)}, Mid: ${index.mid}`);
                    });
                } else if (message.header.messageType === MessageType.TRADE) {
                    const trades = message.body as Trade[];
                    trades.forEach((trade, i) => {
                        console.log(`  Trade ${i}: Ticker 0x${trade.tickerId.toString(16)}, Price: ${trade.price}, Qty: ${trade.quantity}`);
                    });
                }

                client.destroy();
                resolve();
            } catch (error) {
                console.error('Error unpacking message:', error);
                reject(error);
            }
        });

        client.on('error', (err) => {
            console.error('TCP receive error:', err);
            reject(err);
        });
    });
}

// Example: Send Trade message over WebSocket
async function sendTradeWebSocket(): Promise<void> {
    return new Promise((resolve, reject) => {
        const ws = new WebSocket('ws://localhost:8082');

        ws.on('open', () => {
            // Create trade message
            const header: MitchHeader = {
                messageType: MessageType.TRADE,
                timestamp: 1234567890123456n,
                count: 1
            };

            const trade: Trade = {
                tickerId: 0x03006F301CD00000n, // EUR/USD
                price: 1.08750,
                quantity: 100000,
                tradeId: 12345,
                side: 0 // Buy
            };

            const message: MitchMessage = {
                header,
                body: [trade]
            };

            // Serialize using reference implementation
            const bytes = packMitchMessage(message);
            ws.send(bytes);

            console.log(`Sent Trade message via WebSocket successfully (${bytes.length} bytes)`);
            ws.close();
            resolve();
        });

        ws.on('error', (err) => {
            console.error('WebSocket error:', err);
            reject(err);
        });
    });
}

// Example: Subscribe to WebSocket messages
async function subscribeWebSocket(): Promise<void> {
    return new Promise((resolve, reject) => {
        const ws = new WebSocket('ws://localhost:8082');

        ws.on('open', () => {
            console.log('Connected to WebSocket server, waiting for messages...');
        });

        ws.on('message', (data) => {
            try {
                // Deserialize using reference implementation
                const message = unpackMitchMessage(new Uint8Array(data as Buffer));

                console.log(`Received WebSocket message type: ${String.fromCharCode(message.header.messageType)}`);

                if (message.header.messageType === MessageType.TRADE) {
                    const trades = message.body as Trade[];
                    trades.forEach(trade => {
                        console.log(`  Trade: ${trade.price} @ ${trade.quantity}`);
                    });
                }
            } catch (error) {
                console.error('Error unpacking WebSocket message:', error);
            }
        });

        ws.on('error', (err) => {
            console.error('WebSocket subscription error:', err);
            reject(err);
        });

        // Close after 5 seconds for demo
        setTimeout(() => {
            ws.close();
            resolve();
        }, 5000);
    });
}

async function main() {
    console.log('MITCH Protocol TypeScript Examples');

    try {
        console.log('1. Sending Index over TCP');
        await sendIndexTcp();

        console.log('2. Receiving messages over TCP');
        await receiveTcp();

        console.log('3. Sending Trade over WebSocket');
        await sendTradeWebSocket();

        console.log('4. Subscribing to WebSocket messages');
        await subscribeWebSocket();

        console.log('Examples completed!');
    } catch (error) {
        console.error('Example failed:', error);
    }
}

if (require.main === module) {
    main();
}
