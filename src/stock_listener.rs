use lapin::{
    options::*, types::FieldTable, Connection, ConnectionProperties,
};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast::Sender, RwLock};
use crate::models::Stock;

pub type StockStore = Arc<RwLock<HashMap<String, Stock>>>;

pub async fn run_stock_listener(
    tx: Sender<Stock>,
    stock_store: StockStore,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to RabbitMQ
    let conn = Connection::connect(
        "amqp://guest:guest@localhost:5672/%2f",
        ConnectionProperties::default(),
    )
    .await?;
    // Open a channel
    let channel = conn.create_channel().await?;
    // Declare the "stocks" queue
    channel
        .queue_declare(
            "stocks",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;
    // Start consuming messages
    let mut consumer = channel
        .basic_consume(
            "stocks",
            "stock_listener",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;
    println!("Waiting for stock updates...");
    while let Some(result) = consumer.next().await {
        match result {
            Ok(delivery) => {
                // Deserialize the received data into Vec<Stock>
                let stocks: Vec<Stock> = serde_json::from_slice(&delivery.data)?;
                // Acknowledge the message
                delivery
                    .ack(BasicAckOptions::default())
                    .await
                    .expect("Failed to ack");
                // Update the stock_store
                {
                    let mut store = stock_store.write().await;
                    for stock in stocks {
                        let symbol = stock.symbol.clone();
                        if let Some(existing_stock) = store.get(&symbol) {
                            if (existing_stock.price - stock.price).abs() > f64::EPSILON {
                                store.insert(symbol.clone(), stock.clone());
                                println!("Updated stock: {:?}", stock);
                            }
                        } else {
                            store.insert(symbol.clone(), stock.clone());
                            println!("Added new stock: {:?}", stock);
                        }
                        // Broadcast the stock update
                        if let Err(e) = tx.send(stock.clone()) {
                            eprintln!("Error broadcasting stock update: {:?}", e);
                        }
                    }
                }
            }
            Err(error) => {
                eprintln!("Error while receiving message: {:?}", error);
            }
        }
    }
    Ok(())
}