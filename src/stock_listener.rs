use crate::models::Stock;

use lapin::{
    options::*, types::FieldTable, Connection, ConnectionProperties,
    ExchangeKind,
};
use tokio::sync::broadcast;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use futures::StreamExt; // Import the StreamExt trait



pub type StockStore = Arc<RwLock<HashMap<String, Stock>>>;

pub async fn run_stock_listener(
    tx: broadcast::Sender<Stock>,
    stock_store: StockStore,
) -> Result<(), Box<dyn std::error::Error>> {
    // Open a connection to RabbitMQ server
    let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default()).await?;
    println!("Connected to RabbitMQ");

    let channel = conn.create_channel().await?;
    println!("Channel created");

    // Declare an exchange
    channel.exchange_declare(
        "stocks",
        ExchangeKind::Fanout,
        ExchangeDeclareOptions::default(),
        FieldTable::default(),
    ).await?;
    println!("Exchange declared");

    // Declare a queue
    let queue = channel.queue_declare(
        "",
        QueueDeclareOptions::default(),
        FieldTable::default(),
    ).await?;
    println!("Queue declared: {:?}", queue.name());

    // Bind the queue to the exchange
    channel.queue_bind(
        queue.name().as_str(),
        "stocks",
        "",
        QueueBindOptions::default(),
        FieldTable::default(),
    ).await?;
    println!("Queue bound to exchange");

    // Consume messages from the queue
    let mut consumer = channel.basic_consume(
        queue.name().as_str(),
        "",
        BasicConsumeOptions::default(),
        FieldTable::default(),
    ).await?;
    println!("Consumer created");

    while let Some(delivery) = consumer.next().await {
        let delivery = delivery.expect("error in consumer");
        let stock: Stock = serde_json::from_slice(&delivery.data)?;

        // Log the received stock update
        println!("Received stock update: {:?}", stock);

        // Update the stock store
        let mut store = stock_store.write().await;
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

    Ok(())
}