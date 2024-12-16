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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Initialize 5 default stocks
    let mut stocks = vec![
        Stock {
            symbol: "AAPL".to_string(),
            price: 150.0,
            price_change: PriceChange { percentage: 0.0, absolute: 0.0 },
        },
        Stock {
            symbol: "GOOGL".to_string(),
            price: 2800.0,
            price_change: PriceChange { percentage: 0.0, absolute: 0.0 },
        },
        Stock {
            symbol: "AMZN".to_string(),
            price: 3400.0,
            price_change: PriceChange { percentage: 0.0, absolute: 0.0 },
        },
        Stock {
            symbol: "MSFT".to_string(),
            price: 300.0,
            price_change: PriceChange { percentage: 0.0, absolute: 0.0 },
        },
        Stock {
            symbol: "TSLA".to_string(),
            price: 700.0,
            price_change: PriceChange { percentage: 0.0, absolute: 0.0 },
        },
    ];

    let mut rng = rand::thread_rng();

    loop {
        for stock in &mut stocks {
            // Simulate stock price change
            let price_change = rng.gen_range(-5.0..5.0);
            stock.price += price_change;
            stock.price_change = PriceChange {
                percentage: (price_change / stock.price) * 100.0,
                absolute: price_change,
            };

            // Serialize the stock
            let payload = serde_json::to_vec(stock)?;

            // Publish the stock update
            channel.basic_publish(
                "stocks",
                "",
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            ).await?;

            // Log the stock update
            println!("Sent stock update: {:?}", stock);
        }

        // Wait for 1 second before sending the next update
        time::sleep(Duration::from_secs(1)).await;
    }
}