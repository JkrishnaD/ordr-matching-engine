use std::sync::{Arc, Mutex, atomic::AtomicU64};

use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    book::OrderBook,
    handlers::orders::{get_orderbook, submit_order},
};
mod orders;

#[derive(Debug, Clone)]
pub struct OrderState {
    pub book: Arc<Mutex<OrderBook>>,
    pub next_id: Arc<AtomicU64>,
}

pub fn order_routers(state: OrderState) -> Router {
    Router::new()
        .route("/orderbook", get(get_orderbook))
        .route("/orders", post(submit_order))
        .with_state(state)
}
