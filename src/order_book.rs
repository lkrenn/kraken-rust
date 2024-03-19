use serde_json::Value;

#[derive(Debug, Clone)]
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
    fn new(depth: usize) -> Self {
        OrderBook {
            depth,
            bids: Vec::with_capacity(depth),
            asks: Vec::with_capacity(depth),
        }
    }

    // Initializes the order book with a snapshot
    fn initialize(&mut self, snapshot: &Value) {
        // Extracts the snapshot data from the received message format
        if let Some(snapshot_data) = snapshot.get(1) {
            // Parsing asks
            if let Some(asks) = snapshot_data.get("as").and_then(Value::as_array) {
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
            if let Some(bids) = snapshot_data.get("bs").and_then(Value::as_array) {
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
        }

        // Sort bids and asks
        self.bids
            .sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        self.asks
            .sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
    }

    // Updates the order book with changes
    fn update(&mut self, update: &Value) {
        // TODO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::get;
    // Import all functions from the outer module.
    use serde_json::Value;

    fn get_snapshot() -> Value {
        return serde_json::json!(
            [0,{"as":[
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
}
