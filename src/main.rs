use tokio::{net::TcpListener, sync::broadcast};

use crate::{handlers::OrderState, states::Fill, utils::shutdown_signal};

mod book;
mod handlers;
mod matcher;
mod states;
mod subscriber;
mod utils;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let (tx, _rx) = broadcast::channel::<Fill>(1024);

    // creating redis connection manager
    let client = redis::Client::open("redis://127.0.0.1:6379").expect("Invalid redis url");
    let conn = redis::aio::ConnectionManager::new(client.clone())
        .await
        .expect("Failed to connect with redis client");

    tokio::spawn(subscriber::run_fills_subscriber(client.clone(), tx.clone()));

    if std::env::args().any(|a| a == "--matcher") {
        tracing::info!("starting as MATCHER instance");
        tokio::select! {
            _ = matcher::run_matcher(conn.clone()) => {},
            _ = shutdown_signal() => {},
        }
        return;
    };

    tracing::info!("starting as GATEWAY instance");

    let state = OrderState {
        sender: tx,
        redis: conn,
    };
    let app = handlers::order_routers(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    tracing::info!("Server is running at port: {}", port);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
