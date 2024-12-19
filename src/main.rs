use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, Barrier, Mutex};
use tokio::time::{sleep, Duration, timeout}; // Import the sleep, Duration, and timeout modules
use futures::future::join_all; // Import join_all from the futures crate

mod stock_listener;
use stock_listener::{run_stock_listener, StockStore};

mod brokers;
use brokers::run_brokers;

mod traders;
use crate::traders::Trader;
mod models;
mod portfolio;
use portfolio::display_all_portfolios;
mod color; // Add this line to reference the color module

mod stock_send;
use stock_send::run_stock_send;

mod order_sender;
use order_sender::run_order_sender;

mod order_status_receiver;
use order_status_receiver::run_order_status_receiver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, _rx) = broadcast::channel(16);
    // Initialize the stock store
    let stock_store: StockStore = Arc::new(RwLock::new(HashMap::new()));

    let barrier = Arc::new(Barrier::new(6)); // 5 brokers + 1 for the main task

    let tx_clone = tx.clone();
    let stock_store_clone = stock_store.clone();
    let barrier_clone = barrier.clone();

    // Create traders
    let traders: Vec<Arc<Mutex<Trader>>> = vec![
        Arc::new(Mutex::new(Trader::new("B001-T001".to_string()))),
        Arc::new(Mutex::new(Trader::new("B001-T002".to_string()))),
        Arc::new(Mutex::new(Trader::new("B001-T003".to_string()))),
        Arc::new(Mutex::new(Trader::new("B002-T001".to_string()))),
        Arc::new(Mutex::new(Trader::new("B002-T002".to_string()))),
        Arc::new(Mutex::new(Trader::new("B002-T003".to_string()))),
        Arc::new(Mutex::new(Trader::new("B003-T001".to_string()))),
        Arc::new(Mutex::new(Trader::new("B003-T002".to_string()))),
        Arc::new(Mutex::new(Trader::new("B003-T003".to_string()))),
        Arc::new(Mutex::new(Trader::new("B004-T001".to_string()))),
        Arc::new(Mutex::new(Trader::new("B004-T002".to_string()))),
        Arc::new(Mutex::new(Trader::new("B004-T003".to_string()))),
        Arc::new(Mutex::new(Trader::new("B005-T001".to_string()))),
        Arc::new(Mutex::new(Trader::new("B005-T002".to_string()))),
        Arc::new(Mutex::new(Trader::new("B005-T003".to_string()))),
        // Add more traders as needed
    ];

    // Clone the traders vector before passing it to run_brokers
    let traders_clone = traders.clone();


    // Run brokers
    let brokers_handle = tokio::spawn(async move {
        run_brokers(tx, barrier, traders_clone).await;
    });

    // Wait for all brokers to start
    barrier_clone.wait().await;

    //sleep(Duration::from_secs(2)).await;

    // Start the stock sender
    let stock_send_handle = tokio::spawn(async move {
        if let Err(e) = run_stock_send().await {
            eprintln!("RabbitMQ Sender Error: {:?}", e);
        }
    });

    // Spawn the stock listener asynchronously
    let stock_listener_handle = tokio::spawn(async move {
        if let Err(e) = run_stock_listener(tx_clone, stock_store_clone).await {
            eprintln!("RabbitMQ Listener Error: {:?}", e);
        }
    });

    // Start the order sender
    let order_sender_handle = tokio::spawn(async move {
        if let Err(e) = run_order_sender().await {
            eprintln!("RabbitMQ Order Sender Error: {:?}", e);
        }
    });

    // Start the order status receiver
    let order_status_receiver_handle = tokio::spawn(async move {
        if let Err(e) = run_order_status_receiver().await {
            eprintln!("RabbitMQ Order Status Receiver Error: {:?}", e);
        }
    });

    // Run the system for 60 seconds
    let result = timeout(Duration::from_secs(60), async {
        tokio::signal::ctrl_c().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    }).await;

    match result {
        Ok(_) => println!("Trading Market Closed due to stock provider ran away"),
        Err(_) => println!("Trading Market Closed at the end day."),
    }

    // Stop the stock listener and brokers
    stock_listener_handle.abort();
    stock_send_handle.abort();
    order_sender_handle.abort();
    order_status_receiver_handle.abort();
    brokers_handle.abort();

    // Wait for the tasks to be aborted
    let _ = stock_listener_handle.await;
    let _ = stock_send_handle.await;
    let _ = order_sender_handle.await;
    let _ = order_status_receiver_handle.await;
    let _ = brokers_handle.await;

    // Sleep for 3 seconds before managing pending orders
    sleep(Duration::from_secs(1)).await;
    println!("Broker managing pending orders returned to Trader's cash...");
    sleep(Duration::from_secs(2)).await;

     // Cancel pending orders and return cash to traders
     for trader in &traders {
        let mut trader = trader.lock().await;
        trader.cancel_pending_orders();
    }

    // Sleep for 5 seconds before displaying portfolios
    println!("Marketing closing generating all trader performance...");
    sleep(Duration::from_secs(5)).await;

    // Display all trader portfolios
    let trader_refs: Vec<_> = join_all(traders.iter().map(|t| async {
        t.lock().await.clone()
    })).await;
    display_all_portfolios(&trader_refs,  &stock_store).await;

    Ok(())
}