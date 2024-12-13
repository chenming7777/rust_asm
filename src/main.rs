use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, Barrier};

mod stock_listener;
use stock_listener::{run_stock_listener, StockStore};

mod brokers;
use brokers::run_brokers;

mod traders;
mod models;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, _rx) = broadcast::channel(16);
    let stock_store: StockStore = Arc::new(RwLock::new(HashMap::new()));
    let barrier = Arc::new(Barrier::new(6)); // 5 brokers + 1 for the main task

    let tx_clone = tx.clone();
    let stock_store_clone = stock_store.clone();
    let barrier_clone = barrier.clone();

    // Spawn the stock listener asynchronously
    tokio::spawn(async move {
        if let Err(e) = run_stock_listener(tx_clone, stock_store_clone).await {
            eprintln!("RabbitMQ Listener Error: {:?}", e);
        }
    });

    // Run brokers
    run_brokers(tx, barrier).await;

    // Wait for all brokers to start
    barrier_clone.wait().await;

    // Prevent the main function from exiting
    tokio::signal::ctrl_c().await?;
    println!("Shutting down...");

    Ok(())
}