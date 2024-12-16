use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Barrier, mpsc, Mutex};
use tokio::time::{sleep, Duration};
use crate::models::{Stock, OrderType};
use crate::traders::{run_trader, Trader};
use crate::color::print_colored; // Import the print_colored function

// Broker function: Handles stock updates and broadcasts them to traders
pub async fn run_brokers(tx: broadcast::Sender<Stock>, barrier: Arc<Barrier>, traders: Vec<Arc<Mutex<Trader>>>) {
    for i in 0..5 {
        let broker_id = format!("B{:03}", i + 1);
        let mut stock_rx = tx.subscribe(); // Subscribe each broker to the broadcast channel
        let barrier_clone = barrier.clone();

        // Create channels for the broker to communicate with its traders
        let (trader_tx1, trader_rx1) = mpsc::channel(16);
        let (trader_tx2, trader_rx2) = mpsc::channel(16);
        let (trader_tx3, trader_rx3) = mpsc::channel(16);

        // Create a channel for orders from traders to the broker
        let (order_tx, mut order_rx) = mpsc::channel(16);

        // Assign traders to this broker
        let trader1 = traders[i * 3].clone();
        let trader2 = traders[i * 3 + 1].clone();
        let trader3 = traders[i * 3 + 2].clone();

        // Maintain a HashMap of stock symbols to their latest prices
        let mut stock_prices: HashMap<String, f64> = HashMap::new();

        // Spawn the broker task
        tokio::spawn(async move {
            print_colored(&format!("Broker {} started.", broker_id), "cyan");

            // Wait at the barrier
            barrier_clone.wait().await;

            // Spawn three traders for each broker
            tokio::spawn(run_trader(format!("{}-T001", broker_id), trader_rx1, order_tx.clone(), trader1.clone()));
            tokio::spawn(run_trader(format!("{}-T002", broker_id), trader_rx2, order_tx.clone(), trader2.clone()));
            tokio::spawn(run_trader(format!("{}-T003", broker_id), trader_rx3, order_tx.clone(), trader3.clone()));

            loop {
                tokio::select! {
                    stock = stock_rx.recv() => {
                        match stock {
                            Ok(stock) => {
                                print_colored(&format!(
                                    "Broker {} received stock: Symbol: {}, Price: ${:.2}",
                                    broker_id, stock.symbol, stock.price
                                ), "green");

                                // Update the latest stock price
                                stock_prices.insert(stock.symbol.clone(), stock.price);

                                // Forward the stock update to traders
                                if let Err(e) = trader_tx1.send(stock.clone()).await {
                                    print_colored(&format!("Broker {} failed to send stock update to trader 1: {:?}", broker_id, e), "red");
                                }
                                if let Err(e) = trader_tx2.send(stock.clone()).await {
                                    print_colored(&format!("Broker {} failed to send stock update to trader 2: {:?}", broker_id, e), "red");
                                }
                                if let Err(e) = trader_tx3.send(stock.clone()).await {
                                    print_colored(&format!("Broker {} failed to send stock update to trader 3: {:?}", broker_id, e), "red");
                                }

                                // Simulate broadcasting to traders
                                sleep(Duration::from_millis(100)).await;
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                                print_colored(&format!(
                                    "Broker {} lagged, missed {} messages", broker_id, count
                                ), "yellow");
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                print_colored(&format!("Broker {} channel closed", broker_id), "red");
                                break;
                            }
                        }
                    }
                    order = order_rx.recv() => {
                        match order {
                            Some(order) => {
                                let color = match order.order_type {
                                    OrderType::MarketBuy | OrderType::LimitBuy => "green",
                                    OrderType::MarketSell | OrderType::LimitSell => "red",
                                };
                                print_colored(&format!(
                                    "Broker {} received order from Trader {}: {:?} {} shares of {}",
                                    broker_id, order.trader_id, order.order_type, order.quantity, order.stock_symbol
                                ), color);
                                match order.order_type {
                                    OrderType::MarketBuy | OrderType::LimitBuy => {
                                        // Immediately process buy orders
                                        print_colored(&format!(
                                            "Broker {} processed buy order from Trader {}: {} shares of {}",
                                            broker_id, order.trader_id, order.quantity, order.stock_symbol
                                        ), "green");
                                    }
                                    OrderType::MarketSell | OrderType::LimitSell => {
                                        // Immediately process sell orders
                                        print_colored(&format!(
                                            "Broker {} processed sell order from Trader {}: {} shares of {}",
                                            broker_id, order.trader_id, order.quantity, order.stock_symbol
                                        ), "red");
                                    }
                                }
                                // Complete the order
                                let trader = match &order.trader_id[..5] {
                                    "B001-" => trader1.clone(),
                                    "B002-" => trader2.clone(),
                                    "B003-" => trader3.clone(),
                                    _ => continue,
                                };
                                let mut trader = trader.lock().await;
                                if let Some(&stock_price) = stock_prices.get(&order.stock_symbol) {
                                    if let Err(e) = trader.complete_order(&order, stock_price) {
                                        print_colored(&format!("Broker {} failed to complete order for Trader {}: {:?}", broker_id, order.trader_id, e), "red");
                                    } else {
                                        trader.remove_pending_order(&order);
                                    }
                                } else {
                                    print_colored(&format!("Broker {} could not find stock price for order from Trader {}", broker_id, order.trader_id), "red");
                                }
                            }
                            None => {
                                print_colored(&format!("Broker {} order channel closed", broker_id), "red");
                                break;
                            }
                        }
                    }
                }
            }
        });
    }
}