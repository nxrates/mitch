/*!
 * Concise MITCH Protocol Example in Rust
 * Demonstrates usage of mitch.rs reference implementation
 * for sending/receiving messages over TCP and UDP
 */

use std::net::{TcpStream, UdpSocket};
use std::io::{Read, Write};

// Import reference implementation
use crate::mitch::*;

// Example: Send Index message over TCP
fn send_index_tcp() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;

    // Create message using reference implementation
    let header = MitchHeader {
        message_type: 105, // 'i' for Index
        timestamp: 1234567890123456,
        count: 1,
    };

    let index = Index {
        ticker_id: 0x03006F301CD00000, // EUR/USD
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
        accepted: 10,
    };

    let message = MitchMessage::Index {
        header,
        body: vec![index],
    };

    // Serialize using reference implementation
    let bytes = message.to_bytes()?;
    stream.write_all(&bytes)?;

    println!("Sent Index message successfully ({} bytes)", bytes.len());
    Ok(())
}

// Example: Receive messages over TCP
fn receive_tcp() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    let mut buffer = vec![0u8; 1024];

    let bytes_read = stream.read(&mut buffer)?;
    buffer.truncate(bytes_read);

    // Deserialize using reference implementation
    let message = MitchMessage::from_bytes(&buffer)?;

    match message {
        MitchMessage::Index { header, body } => {
            println!("Received Index message with {} entries", header.count);
            for index in body {
                println!("  Ticker: 0x{:016x}, Mid: {:.5}",
                        index.ticker_id, index.mid);
            }
        }
        MitchMessage::Trade { header, body } => {
            println!("Received Trade message with {} entries", header.count);
            for trade in body {
                println!("  Ticker: 0x{:016x}, Price: {:.5}, Qty: {}",
                        trade.ticker_id, trade.price, trade.quantity);
            }
        }
        _ => println!("Received other message type"),
    }

    Ok(())
}

// Example: Send Trade message over UDP
fn send_trade_udp() -> Result<(), Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("127.0.0.1:8081")?;

    // Create trade message
    let header = MitchHeader {
        message_type: 116, // 't' for Trade
        timestamp: 1234567890123456,
        count: 1,
    };

    let trade = Trade {
        ticker_id: 0x03006F301CD00000, // EUR/USD
        price: 1.08750,
        quantity: 100000,
        trade_id: 12345,
        side: 0, // Buy
    };

    let message = MitchMessage::Trade {
        header,
        body: vec![trade],
    };

    // Serialize using reference implementation
    let bytes = message.to_bytes()?;
    socket.send(&bytes)?;

    println!("Sent Trade message via UDP successfully ({} bytes)", bytes.len());
    Ok(())
}

fn main() {
    println!("MITCH Protocol Rust Examples");

    println!("1. Sending Index over TCP");
    if let Err(e) = send_index_tcp() {
        eprintln!("Error: {}", e);
    }

    println!("2. Receiving messages over TCP");
    if let Err(e) = receive_tcp() {
        eprintln!("Error: {}", e);
    }

    println!("3. Sending Trade over UDP");
    if let Err(e) = send_trade_udp() {
        eprintln!("Error: {}", e);
    }

    println!("Examples completed!");
}
