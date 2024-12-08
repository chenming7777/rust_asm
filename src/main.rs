use tokio::sync::broadcast;
use amiquip::Result;
use tokio::time::{sleep, Duration};

mod stock_listener;
use stock_listener::run_stock_listener;

// Import the brokers module
mod brokers;
use brokers::run_brokers;

// Import the models module to use the Stock struct
mod models;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a broadcast channel to communicate stock price updates to all brokers
    let (tx, _rx) = broadcast::channel(16);

    // Spawn a task to listen to RabbitMQ for stock prices
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        if let Err(e) = run_stock_listener(tx_clone).await {
            eprintln!("RabbitMQ Listener Error: {:?}", e);
        }
    });

    // Run brokers in parallel - pass the sender part of the broadcast channel
    run_brokers(tx).await;

    // Keep the program alive
    loop {
        sleep(Duration::from_secs(10)).await;
    }
}
