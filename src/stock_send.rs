use lapin::{
    options::*,
    types::FieldTable,
    BasicProperties, Connection, ConnectionProperties, ExchangeKind,
};
use serde::{Deserialize, Serialize};
use tokio::time;
use std::time::Duration;
use rand::Rng;

#[derive(Serialize, Debug, Deserialize, Clone)]
struct Stock {
    symbol: String,
    price: f64,
    price_change: PriceChange,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
struct PriceChange {
    percentage: f64,
    absolute: f64,
}

pub async fn run_stock_send() -> Result<(), Box<dyn std::error::Error>> {
    // Open a connection to RabbitMQ server
    let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;

    // Declare an exchange
    channel.exchange_declare(
        "stocks",
        ExchangeKind::Fanout,
        ExchangeDeclareOptions::default(),
        FieldTable::default(),
    ).await?;

    // Initialize 60 default stocks
    let stock_symbols = vec![
        "AAPL", "GOOGL", "AMZN", "MSFT", "TSLA", "FB", "NFLX", "NVDA", "BABA", "V",
        "JPM", "JNJ", "WMT", "PG", "MA", "DIS", "HD", "PYPL", "BAC", "VZ",
        "ADBE", "CMCSA", "PFE", "KO", "PEP", "INTC", "CSCO", "MRK", "XOM", "NKE",
        "T", "ABT", "CVX", "LLY", "MCD", "MDT", "UNH", "WFC", "BMY", "COST",
        "NEE", "PM", "HON", "IBM", "TXN", "LIN", "UNP", "QCOM", "LOW", "ORCL",
        "SBUX", "RTX", "CAT", "GS", "MS", "BLK", "AMGN", "SPGI", "PLD", "TMO"
    ];

    let mut stocks: Vec<Stock> = stock_symbols.iter().map(|&symbol| {
        Stock {
            symbol: symbol.to_string(),
            price: 100.0,
            price_change: PriceChange { percentage: 0.0, absolute: 0.0 },
        }
    }).collect();

    // Simulate stock price updates
    loop {
        for stock in &mut stocks {
            let change = rand::thread_rng().gen_range(-5.0..5.0);
            stock.price += change;
            stock.price_change = PriceChange {
                percentage: (change / stock.price) * 100.0,
                absolute: change,
            };

            let payload = serde_json::to_vec(&stock)?;
            channel.basic_publish(
                "stocks",
                "",
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            ).await?;
        }

        time::sleep(Duration::from_secs(1)).await;
    }
}