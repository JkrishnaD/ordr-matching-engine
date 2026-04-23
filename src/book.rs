use crate::states::{Fill, Order, Side, Snapshot, State};
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

    pub fn match_orders(&mut self, mut taker: Order) -> Vec<Fill> {
        let mut fills = Vec::new();

        match taker.side {
            Side::Buy => self.match_buy(&mut taker, &mut fills),
            Side::Sell => self.match_sell(&mut taker, &mut fills),
        };

        // If taker has qty left, rest it on its own side
        if taker.qty > 0 {
            let side = match taker.side {
                Side::Buy => &mut self.bids,
                Side::Sell => &mut self.asks,
            };
            side.entry(taker.price).or_default().push_back(taker);
        }

        fills
    }

    // method to match a buy order
    fn match_buy(&mut self, taker: &mut Order, fills: &mut Vec<Fill>) {
        while taker.qty > 0 {
            // getting the best bid from the order book
            let mut best = match self.asks.first_entry() {
                Some(entry) => entry,
                None => break,
            };

            // compare the best bid price with the taker's price
            let ask_price = *best.key();
            if ask_price > taker.price {
                break; // break if spread is too wide
            }

            // get the queue of orders at the best bid price
            let queue = best.get_mut();
            let maker = queue
                .front_mut()
                .expect("price level should never hold a empty queue");

            // get the trade quantity
            let trade_qty = taker.qty.min(maker.qty);

            // fill the order
            let fill = Fill {
                maker_order_id: maker.id,
                taker_order_id: taker.id,
                price: ask_price,
                qty: trade_qty,
            };
            fills.push(fill);

            // update the maker's and taker's quantities
            maker.qty -= trade_qty;
            taker.qty -= trade_qty;

            // remove the maker's order if it is fully filled
            if maker.qty == 0 {
                queue.pop_front();
                if queue.is_empty() {
                    best.remove();
                }
            }
        }
    }

    fn match_sell(&mut self, taker: &mut Order, fills: &mut Vec<Fill>) {
        while taker.qty > 0 {
            let mut best = match self.bids.last_entry() {
                Some(entry) => entry,
                None => break,
            };

            let bid_price = *best.key();
            if bid_price < taker.price {
                break;
            }

            let queue = best.get_mut();
            let maker = queue
                .front_mut()
                .expect("price level should never hold a empty queue");

            let trade_qty = taker.qty.min(maker.qty);
            let fill = Fill {
                maker_order_id: maker.id,
                taker_order_id: taker.id,
                price: bid_price,
                qty: trade_qty,
            };
            fills.push(fill);
            taker.qty -= trade_qty;
            maker.qty -= trade_qty;

            if maker.qty == 0 {
                queue.pop_front();
                if queue.is_empty() {
                    best.remove();
                }
            }
        }
    }
}

#[test]
pub fn resting_only_no_match() {
    let mut book = OrderBook::new();
    // bid order with no matching ask
    let fills = book.match_orders(Order {
        id: 1,
        side: Side::Buy,
        price: 100,
        qty: 5,
    });
    // assert no fills were generated as there is no matching ask
    assert!(fills.is_empty());
    // assert the bid order is resting on the book
    let snap = book.snapshot();
    assert_eq!(snap.bids.len(), 1);
    assert_eq!(snap.bids[0].price, 100);
    assert_eq!(snap.bids[0].qty, 5);
}

#[test]
pub fn exact_match_both_filled() {
    let mut book = OrderBook::new();
    book.match_orders(Order {
        id: 1,
        side: Side::Sell,
        price: 100,
        qty: 2,
    });
    let fills = book.match_orders(Order {
        id: 2,
        side: Side::Buy,
        price: 100,
        qty: 2,
    });
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].maker_order_id, 1);
    assert_eq!(fills[0].taker_order_id, 2);
    assert_eq!(fills[0].price, 100);
    assert_eq!(fills[0].qty, 2);
    let snap = book.snapshot();
    assert!(snap.bids.is_empty());
    assert!(snap.asks.is_empty());
}

#[test]
pub fn fills_at_maker_not_taker() {
    let mut book = OrderBook::new();
    book.match_orders(Order {
        id: 1,
        side: Side::Sell,
        price: 99,
        qty: 2,
    });
    let fills = book.match_orders(Order {
        id: 2,
        side: Side::Buy,
        price: 100,
        qty: 2,
    });
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].price, 99);
}

#[test]
pub fn matching_multiple_levels() {
    let mut book = OrderBook::new();
    book.match_orders(Order {
        id: 1,
        side: Side::Sell,
        price: 99,
        qty: 2,
    });
    book.match_orders(Order {
        id: 2,
        side: Side::Sell,
        price: 100,
        qty: 3,
    });
    let fills = book.match_orders(Order {
        id: 3,
        side: Side::Buy,
        price: 100,
        qty: 4,
    });
    assert_eq!(fills.len(), 2);
    assert_eq!(fills[0].price, 99);
    assert_eq!(fills[0].qty, 2);
    assert_eq!(fills[1].price, 100);
    assert_eq!(fills[1].qty, 2);
    let snap = book.snapshot();
    assert!(snap.asks[0].qty == 1);
}
