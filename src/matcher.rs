use std::time::Duration;

use redis::{AsyncCommands, aio::ConnectionManager};
use tokio::time::sleep;

use crate::{
    book,
    states::Order,
    utils::{FILLS_CHANNEL, ORDERBOOK_SNAPSHOT, ORDERS_QUEUE},
};

pub async fn run_matcher(mut conn: ConnectionManager) {
    // creating a new order booCo
    let mut book = book::OrderBook::new();
    tracing::info!("Matching started");

    loop {
        // waiting for an order to be placed
        let payload: Option<String> = match conn.lpop(ORDERS_QUEUE, None).await {
            Ok(res) => res,
            Err(e) => {
                tracing::error!("Redis LPOP error: {:?}", e);
                sleep(Duration::from_millis(500)).await;
                continue;
            }
        };

        let Some(payload) = payload else {
            continue;
        };

        // parsing the order from the queue
        let order: Order = serde_json::from_str(&payload).unwrap();
        let fills = book.match_orders(order);

        // publishing fills to the channel
        for fill in &fills {
            let json = serde_json::to_string(fill).unwrap();
            let _: () = conn.publish(FILLS_CHANNEL, json).await.unwrap();
        }

        // updating the order book snapshot
        let snap_json = serde_json::to_string(&book.snapshot()).unwrap();
        let _: () = conn.set(ORDERBOOK_SNAPSHOT, snap_json).await.unwrap();
    }
}
