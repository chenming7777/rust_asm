use criterion::{criterion_group, criterion_main, Criterion};
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties, ExchangeKind};
use tokio::runtime::Runtime;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use std::collections::HashMap;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stock {
    pub symbol: String,
    pub price: f64,
    pub price_change: PriceChange,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PriceChange {
    pub percentage: f64,
    pub absolute: f64,
}

pub type StockStore = Arc<RwLock<HashMap<String, Stock>>>;

async fn run_stock_listener_async(
    tx: broadcast::Sender<Stock>,
    stock_store: StockStore,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;
    channel.exchange_declare(
        "stocks",
        ExchangeKind::Fanout,
        ExchangeDeclareOptions::default(),
        FieldTable::default(),
    ).await?;
    let queue = channel.queue_declare(
        "",
        QueueDeclareOptions::default(),
        FieldTable::default(),
    ).await?;
    channel.queue_bind(
        queue.name().as_str(),
        "stocks",
        "",
        QueueBindOptions::default(),
        FieldTable::default(),
    ).await?;
    let mut consumer = channel.basic_consume(
        queue.name().as_str(),
        "",
        BasicConsumeOptions::default(),
        FieldTable::default(),
    ).await?;
    while let Some(delivery) = consumer.next().await {
        let delivery = delivery.expect("error in consumer");
        let stock: Stock = serde_json::from_slice(&delivery.data)?;
        let mut store = stock_store.write().await;
        store.insert(stock.symbol.clone(), stock.clone());
        tx.send(stock.clone()).unwrap();
    }
    Ok(())
}

fn run_stock_listener_sync(
    tx: broadcast::Sender<Stock>,
    stock_store: StockStore,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = lapin::blocking::Connection::connect("amqp://127.0.0.1:5672/%2f", lapin::blocking::ConnectionProperties::default())?;
    let channel = conn.create_channel()?;
    channel.exchange_declare(
        "stocks",
        ExchangeKind::Fanout,
        ExchangeDeclareOptions::default(),
        FieldTable::default(),
    )?;
    let queue = channel.queue_declare(
        "",
        QueueDeclareOptions::default(),
        FieldTable::default(),
    )?;
    channel.queue_bind(
        queue.name().as_str(),
        "stocks",
        "",
        QueueBindOptions::default(),
        FieldTable::default(),
    )?;
    let consumer = channel.basic_consume(
        queue.name().as_str(),
        "",
        BasicConsumeOptions::default(),
        FieldTable::default(),
    )?;
    for delivery in consumer {
        let delivery = delivery?;
        let stock: Stock = serde_json::from_slice(&delivery.data)?;
        let mut store = stock_store.write().unwrap();
        store.insert(stock.symbol.clone(), stock.clone());
        tx.send(stock.clone()).unwrap();
    }
    Ok(())
}

fn benchmark_async(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (tx, _) = broadcast::channel(16);
    let stock_store: StockStore = Arc::new(RwLock::new(HashMap::new()));
    c.bench_function("async stock listener", |b| {
        b.to_async(&rt).iter(|| run_stock_listener_async(tx.clone(), stock_store.clone()))
    });
}

fn benchmark_sync(c: &mut Criterion) {
    let (tx, _) = broadcast::channel(16);
    let stock_store: StockStore = Arc::new(RwLock::new(HashMap::new()));
    c.bench_function("sync stock listener", |b| {
        b.iter(|| run_stock_listener_sync(tx.clone(), stock_store.clone()))
    });
}

criterion_group!(benches, benchmark_async, benchmark_sync);
criterion_main!(benches);