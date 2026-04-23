use axum::{Json, extract::State};
use serde::Serialize;

use crate::{
    handlers::OrderState,
    states::{Fill, Order, OrderRequest, Snapshot},
};

#[axum::debug_handler]
pub async fn get_orderbook(State(state): State<OrderState>) -> Json<Snapshot> {
    tracing::info!("Fetching orders...");
    let book = state.book.lock().unwrap();

    let snapshot = book.snapshot();
    tracing::info!("Orders Fetched");

    Json(snapshot)
}

#[derive(Serialize)]
pub struct PostOrderResponse {
    pub id: u64,
    pub fills: Vec<Fill>,
}

#[axum::debug_handler]
pub async fn submit_order(
    State(state): State<OrderState>,
    Json(req): Json<OrderRequest>,
) -> Json<PostOrderResponse> {
    // assign a unique id to the order
    let id = state
        .next_id
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // building the order
    let order = Order {
        id,
        side: req.side,
        price: req.price,
        qty: req.qty,
    };
    // matching the order against the book
    let mut book = state.book.lock().unwrap();
    let fills = book.match_orders(order);
    tracing::info!("Order Submitted...");
    // returning the response
    Json(PostOrderResponse { id, fills })
}
