use amiquip::{Connection, Exchange, Publish, Result};
use serde::{Deserialize, Serialize};
use std::{thread, time};
use rand::Rng;

fn main() -> Result<()> {
    // Open a connection to RabbitMQ server
    let mut connection = Connection::insecure_open("amqp://guest:guest@localhost:5672")?;

    // Open a channel - None says let the library choose the channel ID.
    let channel = connection.open_channel(None)?;

    // Get a handle to the direct exchange on our channel.
    let exchange = Exchange::direct(&channel);

    // Infinite loop to publish stock updates every 1 seconds
    loop {
        // Define the initial stock data
        let mut stocks = vec![
            Stock {
                symbol: "AAPL".to_string(),
                price: 175.32,
                price_change: PriceChange {
                    percentage: 0.85,
                    absolute: 1.48,
                },
            },
            Stock {
                symbol: "MSFT".to_string(),
                price: 348.15,
                price_change: PriceChange {
                    percentage: 1.12,
                    absolute: 3.87,
                },
            },
            Stock {
                symbol: "GOOGL".to_string(),
                price: 138.22,
                price_change: PriceChange {
                    percentage: 0.98,
                    absolute: 1.34,
                },
            },
            Stock {
                symbol: "AMZN".to_string(),
                price: 143.55,
                price_change: PriceChange {
                    percentage: -0.45,
                    absolute: -0.65,
                },
            },
            Stock {
                symbol: "META".to_string(),
                price: 303.21,
                price_change: PriceChange {
                    percentage: 1.45,
                    absolute: 4.33,
                },
            },
            Stock {
                symbol: "TSLA".to_string(),
                price: 245.67,
                price_change: PriceChange {
                    percentage: -1.23,
                    absolute: -3.06,
                },
            },
            Stock {
                symbol: "NVDA".to_string(),
                price: 478.29,
                price_change: PriceChange {
                    percentage: 2.10,
                    absolute: 9.84,
                },
            },
            Stock {
                symbol: "NFLX".to_string(),
                price: 420.14,
                price_change: PriceChange {
                    percentage: 0.68,
                    absolute: 2.85,
                },
            },
            Stock {
                symbol: "DIS".to_string(),
                price: 88.45,
                price_change: PriceChange {
                    percentage: -0.98,
                    absolute: -0.87,
                },
            },
            Stock {
                symbol: "BAC".to_string(),
                price: 28.95,
                price_change: PriceChange {
                    percentage: 0.35,
                    absolute: 0.10,
                },
            },
        ];

        // Update the price of each stock by -10%, +10%, or remain the same
        for stock in &mut stocks {
            let mut rng = rand::thread_rng();
            let change_factor: f64 = rng.gen_range(-0.10..=0.10); // Generate a random change factor between -10% and +10%

            // Calculate new price
            let new_price = stock.price * (1.0 + change_factor);

            // Update the stock with the new price and set price change details
            let price_change_absolute = new_price - stock.price;
            let price_change_percentage = (price_change_absolute / stock.price) * 100.0;

            stock.price = new_price;
            stock.price_change = PriceChange {
                percentage: price_change_percentage,
                absolute: price_change_absolute,
            };
        }

        // Serialize the stock data to JSON
        let serialized_stocks = serde_json::to_string(&stocks).unwrap();

        // Publish the stock data to the "stocks" queue
        exchange.publish(Publish::new(serialized_stocks.as_bytes(), "stocks")).unwrap();

        println!("Published updated stock prices to the queue.");

        // Wait for 1 seconds before sending the next update
        thread::sleep(time::Duration::from_secs(1));
    }
}

#[derive(Serialize, Debug, Deserialize)]
struct Stock {
    symbol: String,
    price: f64,
    price_change: PriceChange,
}

#[derive(Serialize, Debug, Deserialize)]
struct PriceChange {
    percentage: f64,
    absolute: f64,
}
