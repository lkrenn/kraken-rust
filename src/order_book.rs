use crc32fast::Hasher;
use serde_json::Value;
use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    price: f64,
    volume: f64,
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    depth: usize,
    bids: Vec<Level>,
    asks: Vec<Level>,
}

impl OrderBook {
    pub fn new(depth: usize) -> Self {
        OrderBook {
            depth,
            bids: Vec::with_capacity(depth),
            asks: Vec::with_capacity(depth),
        }
    }

    // Initializes the order book with a snapshot
    pub fn initialize(&mut self, snapshot: &Value) {
        // Parsing asks
        if let Some(asks) = snapshot.get("as").and_then(Value::as_array) {
            self.asks = asks
                .iter()
                .take(self.depth)
                .filter_map(|ask| {
                    let price = ask.get(0)?.as_str()?.parse::<f64>().ok()?;
                    let volume = ask.get(1)?.as_str()?.parse::<f64>().ok()?;
                    Some(Level { price, volume })
                })
                .collect();
        }

        // Parsing bids
        if let Some(bids) = snapshot.get("bs").and_then(Value::as_array) {
            self.bids = bids
                .iter()
                .take(self.depth)
                .filter_map(|bid| {
                    let price = bid.get(0)?.as_str()?.parse::<f64>().ok()?;
                    let volume = bid.get(1)?.as_str()?.parse::<f64>().ok()?;
                    Some(Level { price, volume })
                })
                .collect();
        }

        // Sort bids and asks
        self.bids
            .sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        self.asks
            .sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
    }

    // Updates the order book with changes
    pub fn update(&mut self, update: &serde_json::Value) {
        if let Some(update_data) = update.get(1) {
            // Handle asks update
            if let Some(asks_update) = update_data.get("a").and_then(|a| a.as_array()) {
                for ask in asks_update {
                    if let Some((price_str, volume_str, _timestamp)) =
                        ask.as_array().and_then(|ask| {
                            Some((ask[0].as_str()?, ask[1].as_str()?, ask[2].as_str()?))
                        })
                    {
                        let price: f64 = price_str.parse().unwrap_or(0.0);
                        let volume: f64 = volume_str.parse().unwrap_or(0.0);

                        if volume == 0.0 {
                            // Delete the price level with 0 volume
                            self.asks.retain(|a| a.price != price);
                        } else {
                            // Check if the price level exists and update or insert accordingly
                            match self.asks.iter_mut().find(|a| a.price == price) {
                                Some(existing_ask) => existing_ask.volume = volume, // Update existing
                                None => {
                                    // Insert new price level in sorted order
                                    let new_ask = Level { price, volume };
                                    self.asks.push(new_ask);
                                    self.asks
                                        .sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
                                }
                            }
                        }
                    }
                }
            }

            if let Some(bids_update) = update_data.get("b").and_then(|b| b.as_array()) {
                for bid in bids_update {
                    if let Some((price_str, volume_str, _timestamp)) =
                        bid.as_array().and_then(|bid| {
                            Some((bid[0].as_str()?, bid[1].as_str()?, bid[2].as_str()?))
                        })
                    {
                        let price: f64 = price_str.parse().unwrap_or(0.0);
                        let volume: f64 = volume_str.parse().unwrap_or(0.0);

                        if volume == 0.0 {
                            // Delete the price level with 0 volume
                            self.bids.retain(|b| b.price != price);
                        } else {
                            // Check if the price level exists and update or insert accordingly
                            match self.bids.iter_mut().find(|b| b.price == price) {
                                Some(existing_bid) => existing_bid.volume = volume, // Update existing
                                None => {
                                    // Insert new price level in sorted order
                                    let new_bid = Level { price, volume };
                                    self.bids.push(new_bid);
                                    self.bids
                                        .sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
                                }
                            }
                        }
                    }
                }
            }
        }
        self.truncate_to_depth();
    }

    fn truncate_to_depth(&mut self) {
        // Truncate asks to the specified depth
        if self.asks.len() > self.depth {
            self.asks.truncate(self.depth);
        }
        // Truncate bids to the specified depth
        if self.bids.len() > self.depth {
            self.bids.truncate(self.depth);
        }

        // Since we may have inserted a new price level, ensure the order book is sorted
        self.asks
            .sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
        self.bids
            .sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
    }

    pub fn calculate_checksum(&self) -> u32 {
        let mut input_string = String::new();

        // Process asks
        for ask in self.asks.iter().take(10) {
            let price = format!("{:.5}", ask.price)
                .replace(".", "")
                .trim_start_matches('0')
                .to_string();
            let volume = format!("{:.8}", ask.volume)
                .replace(".", "")
                .trim_start_matches('0')
                .to_string();
            input_string.push_str(&price);
            input_string.push_str(&volume);
        }

        // Process bids
        for bid in self.bids.iter().take(10) {
            // Ensure high to low order for bids
            let price = format!("{:.5}", bid.price)
                .replace(".", "")
                .trim_start_matches('0')
                .to_string();
            let volume = format!("{:.8}", bid.volume)
                .replace(".", "")
                .trim_start_matches('0')
                .to_string();
            input_string.push_str(&price);
            input_string.push_str(&volume);
        }

        let mut hasher = Hasher::new();
        hasher.update(input_string.as_bytes());
        hasher.finalize()
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.5} ({:.8})", self.price, self.volume)
    }
}

impl fmt::Display for OrderBook {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Order Book:")?;
        writeln!(
            f,
            "{:<10} {:<20} | {:<10} {}",
            "Depth", "Bid", "Ask", "Depth"
        )?;
        for i in 0..self.depth {
            let bid_level = self
                .bids
                .get(i)
                .map_or("".to_string(), |level| format!("{}", level));
            let ask_level = self
                .asks
                .get(i)
                .map_or("".to_string(), |level| format!("{}", level));
            writeln!(
                f,
                "{:<10} {:<20} | {:<10} {}",
                i + 1,
                bid_level,
                ask_level,
                i + 1
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn get_snapshot() -> Value {
        return serde_json::json!(
        [0,
        {"as":[
            ["5711.80000","8.13439401","1557070784.848047"],
            ["5712.20000","2.00000000","1557070757.056750"],
            ["5712.80000","0.30000000","1557070783.806432"],
            ["5713.00000","3.29800000","1557070774.281619"],
            ["5713.10000","1.00000000","1557070741.315583"],
            ["5713.90000","1.00000000","1557070698.840502"],
            ["5714.70000","0.50000000","1557070743.861074"],
            ["5715.20000","1.00000000","1557070697.871150"],
            ["5716.60000","1.22700000","1557070775.294557"],
            ["5716.80000","0.35000000","1557070749.823148"]],
        "bs":[
            ["5711.70000","0.00749800","1557070712.848376"],
            ["5709.20000","3.30000000","1557070766.260894"],
            ["5708.30000","0.75483907","1557070781.425374"],
            ["5708.20000","5.00000000","1557070780.762871"],
            ["5707.80000","2.50000000","1557070722.912548"],
            ["5707.40000","4.33000000","1557070732.546143"],
            ["5707.00000","0.00200000","1557070604.962840"],
            ["5706.90000","1.17300000","1557070715.529722"],
            ["5706.40000","0.85600000","1557070777.204262"],
            ["5706.30000","1.00000000","1557070753.118938"]]
        },
        "book-10",
        "XBT/USD"]
        );
    }

    fn get_update1() -> Value {
        return serde_json::json!(
        [0,
        {"b":[
            ["5709.20000","3.00000000","1557070785.898642"],
            ["5708.20000","0.00000000","1557070786.010118"],
            ["5705.90000","7.62400000","1557070783.582385","r"]],
             "c":"2470128591"
        },
        "book-10",
        "XBT/USD"]
        );
    }

    fn get_update2() -> Value {
        return serde_json::json!(
            [0,
            {"b":[
                ["5709.20000","8.00000000","1557070786.250425"],
                ["5709.40000","0.30000000","1557070786.259115"]],
                 "c":"4148072505"},"book-10","XBT/USD"]
        );
    }

    fn get_update3() -> Value {
        return serde_json::json!(
            [0,
            {"b":[
                ["5708.30000","0.00000000","1557070786.389495"],
                ["5705.90000","7.62400000","1557070783.582385","r"]],
                 "c":"3093569863"},"book-10","XBT/USD"]
        );
    }

    fn get_expected_order_book1() -> Value {
        return serde_json::json!(
            [
                0,
                {
                    "as": [
                        ["5711.80000", "8.13439401"],
                        ["5712.20000", "2.00000000"],
                        ["5712.80000", "0.30000000"],
                        ["5713.00000", "3.29800000"],
                        ["5713.10000", "1.00000000"],
                        ["5713.90000", "1.00000000"],
                        ["5714.70000", "0.50000000"],
                        ["5715.20000", "1.00000000"],
                        ["5716.60000", "1.22700000"],
                        ["5716.80000", "0.35000000"]
                    ],
                    "bs": [
                        ["5711.70000", "0.00749800"],
                        ["5709.20000", "3.00000000"],
                        ["5708.30000", "0.75483907"],
                        ["5707.80000", "2.50000000"],
                        ["5707.40000", "4.33000000"],
                        ["5707.00000", "0.00200000"],
                        ["5706.90000", "1.17300000"],
                        ["5706.40000", "0.85600000"],
                        ["5706.30000", "1.00000000"],
                        ["5705.90000", "7.62400000"]
                    ]
                },
                "book-10",
                "XBT/USD"
            ]
        );
    }

    fn get_expected_order_book2() -> Value {
        return serde_json::json!(
            [
                0,
                {
                    "as": [
                        ["5711.80000", "8.13439401"],
                        ["5712.20000", "2.00000000"],
                        ["5712.80000", "0.30000000"],
                        ["5713.00000", "3.29800000"],
                        ["5713.10000", "1.00000000"],
                        ["5713.90000", "1.00000000"],
                        ["5714.70000", "0.50000000"],
                        ["5715.20000", "1.00000000"],
                        ["5716.60000", "1.22700000"],
                        ["5716.80000", "0.35000000"]
                    ],
                    "bs": [
                        ["5711.70000", "0.00749800"],
                        ["5709.40000", "0.30000000"],
                        ["5709.20000", "8.00000000"],
                        ["5708.30000", "0.75483907"],
                        ["5707.80000", "2.50000000"],
                        ["5707.40000", "4.33000000"],
                        ["5707.00000", "0.00200000"],
                        ["5706.90000", "1.17300000"],
                        ["5706.40000", "0.85600000"],
                        ["5706.30000", "1.00000000"]
                    ]
                },
                "book-10",
                "XBT/USD"
            ]
        );
    }

    fn get_expected_order_book3() -> Value {
        return serde_json::json! {
            [0,
            {
                "as": [
                    ["5711.80000", "8.13439401"],
                    ["5712.20000", "2.00000000"],
                    ["5712.80000", "0.30000000"],
                    ["5713.00000", "3.29800000"],
                    ["5713.10000", "1.00000000"],
                    ["5713.90000", "1.00000000"],
                    ["5714.70000", "0.50000000"],
                    ["5715.20000", "1.00000000"],
                    ["5716.60000", "1.22700000"],
                    ["5716.80000", "0.35000000"]
                ],
                "bs": [
                    ["5711.70000", "0.00749800"],
                    ["5709.40000", "0.30000000"],
                    ["5709.20000", "8.00000000"],
                    ["5707.80000", "2.50000000"],
                    ["5707.40000", "4.33000000"],
                    ["5707.00000", "0.00200000"],
                    ["5706.90000", "1.17300000"],
                    ["5706.40000", "0.85600000"],
                    ["5706.30000", "1.00000000"],
                    ["5705.90000", "7.62400000"]
                ]
            },
            "book-10",
            "XBT/USD"
        ]
        };
    }

    #[test]
    fn test_order_book_initialization() {
        let mut order_book = OrderBook::new(10);
        let snapshot = get_snapshot();

        order_book.initialize(&snapshot);

        assert_eq!(order_book.asks.len(), 10);
        assert_eq!(order_book.bids.len(), 10);
        assert!(order_book.asks.iter().all(|level| level.price > 5711.75));
        assert!(order_book.bids.iter().all(|level| level.price < 5711.75));
    }

    #[test]
    fn test_order_book_update() {
        // Initialize the OrderBook with a known snapshot.
        let mut order_book = OrderBook::new(10);
        let initial_snapshot = get_snapshot();

        order_book.initialize(&initial_snapshot);

        // Apply updates to the OrderBook.
        let updates1 = get_update1();
        order_book.update(&updates1);

        // Verify that the OrderBook now matches the expected output.
        let mut expected_order_book = OrderBook::new(10);
        expected_order_book.initialize(&get_expected_order_book1());

        assert_eq!(order_book.asks, expected_order_book.asks);
        assert_eq!(order_book.bids, expected_order_book.bids);

        // Apply another update to the OrderBook
        let updates2 = get_update2();
        order_book.update(&updates2);

        expected_order_book = OrderBook::new(10);
        expected_order_book.initialize(&get_expected_order_book2());

        assert_eq!(order_book.asks, expected_order_book.asks);
        assert_eq!(order_book.bids, expected_order_book.bids);

        // Apply another update to the OrderBook
        let updates3 = get_update3();
        order_book.update(&updates3);

        expected_order_book = OrderBook::new(10);
        expected_order_book.initialize(&get_expected_order_book3());

        assert_eq!(order_book.asks, expected_order_book.asks);
        assert_eq!(order_book.bids, expected_order_book.bids);
    }

    #[test]
    fn test_order_book_checksum() {
        let mut order_book = OrderBook::new(10);
        order_book.initialize(&serde_json::json!(
            [0,
            {
                "as": [
                    [ "0.05005", "0.00000500", "1582905487.684110" ],
                    [ "0.05010", "0.00000500", "1582905486.187983" ],
                    [ "0.05015", "0.00000500", "1582905484.480241" ],
                    [ "0.05020", "0.00000500", "1582905486.645658" ],
                    [ "0.05025", "0.00000500", "1582905486.859009" ],
                    [ "0.05030", "0.00000500", "1582905488.601486" ],
                    [ "0.05035", "0.00000500", "1582905488.357312" ],
                    [ "0.05040", "0.00000500", "1582905488.785484" ],
                    [ "0.05045", "0.00000500", "1582905485.302661" ],
                    [ "0.05050", "0.00000500", "1582905486.157467" ] ],
                "bs": [
                    [ "0.05000", "0.00000500", "1582905487.439814" ],
                    [ "0.04995", "0.00000500", "1582905485.119396" ],
                    [ "0.04990", "0.00000500", "1582905486.432052" ],
                    [ "0.04980", "0.00000500", "1582905480.609351" ],
                    [ "0.04975", "0.00000500", "1582905476.793880" ],
                    [ "0.04970", "0.00000500", "1582905486.767461" ],
                    [ "0.04965", "0.00000500", "1582905481.767528" ],
                    [ "0.04960", "0.00000500", "1582905487.378907" ],
                    [ "0.04955", "0.00000500", "1582905483.626664" ],
                    [ "0.04950", "0.00000500", "1582905488.509872" ] ]
                }
            ]
        ));
        assert_eq!(order_book.calculate_checksum(), 974947235);
    }
}
