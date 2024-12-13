use lapin::{
    options::*,
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind,
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
    let conn = Connection::connect(
        "amqp://guest:guest@localhost:5672/%2f",
        ConnectionProperties::default(),
    )
    .await?;

    // Open a channel
    let channel = conn.create_channel().await?;

    // Declare the "stocks_exchange" exchange
    channel
        .exchange_declare(
            "stocks_exchange",
            ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // Declare the "stocks" queue and bind it to the exchange
    channel
        .queue_declare(
            "stocks",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;
    channel
        .queue_bind(
            "stocks",
            "stocks_exchange",
            "stocks",
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // Define the initial stock data
    let mut stocks = vec![
        Stock {
            symbol: "AAPL".to_string(),
            price: 100.00,
            price_change: PriceChange {
                percentage: 0.0,
                absolute: 0.0,
            },
        },
        // Add other stocks as needed
    ];

    // Publish the initial open market prices of all stocks
    let serialized_stocks = serde_json::to_vec(&stocks)?;
    publish_message(&channel, &serialized_stocks).await?;
    println!("Published open market prices of all stocks.");

    // Infinite loop to publish updates with changed prices only
    loop {
        let mut changed_stocks = Vec::new();

        // Update the price of some stocks randomly
        let mut rng = rand::thread_rng();
        for stock in &mut stocks {
            let should_change = rng.gen_bool(0.2); // 20% chance to change the stock price
            if should_change {
                let change_factor: f64 = rng.gen_range(-0.10..=0.10); // Between -10% and +10%
                let new_price = stock.price * (1.0 + change_factor);
                let price_change_absolute = new_price - stock.price;
                let price_change_percentage = (price_change_absolute / stock.price) * 100.0;

                stock.price = new_price;
                stock.price_change = PriceChange {
                    percentage: price_change_percentage,
                    absolute: price_change_absolute,
                };

                // Add to the list of changed stocks
                changed_stocks.push(stock.clone());
            }
        }

        // Only publish if there are changed stocks
        if !changed_stocks.is_empty() {
            let serialized_changes = serde_json::to_vec(&changed_stocks)?;
            publish_message(&channel, &serialized_changes).await?;
            println!("Published updated stock prices: {:?}", changed_stocks);
        }

        // Wait for 1 second before sending the next update
        time::sleep(Duration::from_secs(1)).await;
    }
}

async fn publish_message(
    channel: &Channel,
    message: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    channel
        .basic_publish(
            "stocks_exchange",
            "stocks",
            BasicPublishOptions::default(),
            message,
            BasicProperties::default(),
        )
        .await?
        .await?; // Wait for the confirmation from the broker

    Ok(())
}