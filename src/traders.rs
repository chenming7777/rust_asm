use tokio::sync::mpsc;
use tokio::sync::Mutex;
use crate::models::{Stock, Order, OrderType, Trader};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;

// Trader function: Listens to its broker's updates and processes stock data
pub async fn run_trader(
    trader_id: String,
    mut stock_rx: mpsc::Receiver<Stock>,
    order_tx: mpsc::Sender<Order>,
) {
    println!("Trader {} started.", trader_id);
    let rng = Arc::new(Mutex::new(StdRng::from_entropy()));
    let mut trader = Trader::new(trader_id.clone());

    loop {
        match stock_rx.recv().await {
            Some(stock) => {
                println!(
                    "Trader {} received stock update: Symbol: {}, Price: ${:.2}",
                    trader_id, stock.symbol, stock.price
                );
                // Simulate decision-making (e.g., market buy/limit buy/hold)
                let mut rng = rng.lock().await;
                let decision = rng.gen_range(0..=2);
                match decision {
                    0 => {
                        // Market buy
                        println!(
                            "Trader {} decided to market buy stock: {}",
                            trader_id, stock.symbol
                        );
                        let quantity = rng.gen_range(1..=10);
                        let order = Order {
                            trader_id: trader_id.clone(),
                            stock_symbol: stock.symbol.clone(),
                            order_type: OrderType::MarketBuy,
                            quantity,
                            limit_price: None,
                        };
                        // Send the order to the broker
                        if let Err(e) = order_tx.send(order).await {
                            eprintln!("Trader {} failed to send order: {:?}", trader_id, e);
                        } else {
                            // Update the trader's portfolio
                            match trader.buy_stock(stock.clone(), quantity) {
                                Ok(_) => println!("Trader {} successfully bought {} shares of {}", trader_id, quantity, stock.symbol),
                                Err(err) => println!("{}", err),
                            }
                        }
                    }
                    1 => {
                        // Limit buy
                        let limit_price = rng.gen_range(stock.price * 0.9..=stock.price * 1.1);
                        println!(
                            "Trader {} decided to limit buy stock: {} at price ${:.2}",
                            trader_id, stock.symbol, limit_price
                        );
                        let quantity = rng.gen_range(1..=10);
                        let order = Order {
                            trader_id: trader_id.clone(),
                            stock_symbol: stock.symbol.clone(),
                            order_type: OrderType::LimitBuy,
                            quantity,
                            limit_price: Some(limit_price),
                        };
                        // Send the order to the broker
                        if let Err(e) = order_tx.send(order).await {
                            eprintln!("Trader {} failed to send order: {:?}", trader_id, e);
                        }
                    }
                    _ => {
                        // Hold
                        println!("Trader {} decided to hold on stock: {}", trader_id, stock.symbol);
                    }
                }
            }
            None => {
                println!("Trader {} channel closed", trader_id);
                break;
            }
        }
    }
}