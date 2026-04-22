use crate::states::{Order, Snapshot, State};
use std::collections::{BTreeMap, VecDeque};

pub struct OrderBook {
    bids: BTreeMap<u64, VecDeque<Order>>,
    asks: BTreeMap<u64, VecDeque<Order>>,
}

impl OrderBook {
    // Initializing new OrderBook with empty maps
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    // Returns a snapshot of the current order book state
    pub fn snapshot(&self) -> Snapshot {
        // bids with highest price first
        let bids = self
            .bids
            .iter()
            .rev()
            .map(|(p, q)| State {
                price: *p,
                qty: q.iter().map(|o| o.qty).sum(),
            })
            .collect();

        // asks with lowest price first
        let asks = self
            .asks
            .iter()
            .map(|(p, q)| State {
                price: *p,
                qty: q.iter().map(|o| o.qty).sum(),
            })
            .collect();

        Snapshot { bids, asks }
    }
}
