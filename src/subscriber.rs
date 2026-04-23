use futures_util::StreamExt;
use redis::Client;
use tokio::sync::broadcast;

use crate::{states::Fill, utils::FILLS_CHANNEL};

pub async fn run_fills_subscriber(client: Client, fills_tx: broadcast::Sender<Fill>) {
    let mut pubsub = client
        .get_async_pubsub()
        .await
        .expect("pubsub connect failed");
    pubsub
        .subscribe(FILLS_CHANNEL)
        .await
        .expect("subscribe failed");
    tracing::info!("fills subscriber started");

    let mut stream = pubsub.on_message();
    while let Some(msg) = stream.next().await {
        let payload: String = match msg.get_payload() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let fill: Fill = match serde_json::from_str(&payload) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let _ = fills_tx.send(fill);
    }
}
