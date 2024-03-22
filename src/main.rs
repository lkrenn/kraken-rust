use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

mod order_book;
mod test_test;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = Url::parse("wss://ws.kraken.com/")?;

    // Connect to the WebSocket server
    let (ws_stream, response) = connect_async(url).await?;

    // Now, correctly split ws_stream into a writer and reader parts
    let (mut write, read) = ws_stream.split();

    // Initialize an empty order book
    let mut order_book = order_book::OrderBook::new(10);

    // Proceed to send messages and read responses
    // For example, to send a subscription message:
    // let subscribe_command = serde_json::json!({
    //     "event": "subscribe",
    //     "pair": ["XBT/USD"],
    //     "subscription": {"name": "trade"}
    // }).to_string();

    let subscribe_command = serde_json::json!({
        "event": "subscribe",
        "pair": ["XBT/USD"],
        "subscription": {
            "name": "book",
            "depth": 10
    }
    })
    .to_string();

    // Send the subscription message
    write.send(Message::Text(subscribe_command)).await?;

    // Process incoming messages
    let mut read = read;
    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Text(text)) => {
                let json_msg: Value = serde_json::from_str(&text)?;
                if let Some(update) = json_msg.get(1) {
                    if update.get("b").is_some() || update.get("a").is_some() {
                        process_order_book_update(&mut order_book, &update);
                    } else if update.get("bs").is_some() && update.get("as").is_some() {
                        // Initialize the order book if bs and as are in the keys
                        order_book.initialize(&update);
                    } else {
                        println!("Unknown message: {}", update);
                    }
                }
                // Assuming heartbeat messages can be distinguished by a lack of "b" or "a" keys
                else {
                    // Handle the heartbeat
                    process_heartbeat(&json_msg);
                }
            }
            Ok(_) => (), // Other message types
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}

// Handle extra messages
fn process_heartbeat(message: &Value) {
    println!("Heartbeat received: {}", message);
}

fn process_order_book_update(order_book: &mut order_book::OrderBook, message: &Value) {
    order_book.update(message);

    if let Some(checksum_str) = message.get("c").and_then(Value::as_str) {
        match checksum_str.parse::<u32>() {
            Ok(checksum) => {
                if order_book.calculate_checksum().eq(&checksum) {
                    println!("Checksum as u32: {}", checksum);
                } else {
                    println!("Checksum does not match!");
                }
            }
            Err(e) => {
                eprintln!("Failed to parse checksum as u32: {}", e);
            }
        }
    } else {
        eprintln!("'c' key not found or not a string");
    }
}
