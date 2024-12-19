use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Barrier, mpsc, Mutex};
use crate::models::Stock; // Import the Order struct
use tokio::time::{sleep, Duration};
use crate::traders::{run_trader, Trader};
use crate::color::print_colored; // Import the print_colored function
use lapin::{Connection, ConnectionProperties, ExchangeKind, BasicProperties, options::*, types::FieldTable};
use serde::{Deserialize, Serialize};
use futures::StreamExt; // Import the StreamExt trait

#[derive(Serialize, Debug, Deserialize)]
struct OrderStatusUpdate {
    order_id: String,
    status: String,
}

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

        // Establish connection to RabbitMQ server for sending orders
        let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default()).await.unwrap();
        let channel = conn.create_channel().await.unwrap();
        channel.exchange_declare(
            "orders",
            ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        ).await.unwrap();

        // Establish connection to RabbitMQ server for receiving order status updates
        let status_conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default()).await.unwrap();
        let status_channel = status_conn.create_channel().await.unwrap();
        let _status_queue = status_channel.queue_declare("processed_order_status", QueueDeclareOptions::default(), FieldTable::default()).await.unwrap();
        let mut status_consumer = status_channel.basic_consume("processed_order_status", "", BasicConsumeOptions::default(), FieldTable::default()).await.unwrap();

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
                                // print_colored(&format!(
                                //     "Broker {} received stock: Symbol: {}, Price: ${:.2}",
                                //     broker_id, stock.symbol, stock.price
                                // ), "green");

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
                                // sleep(Duration::from_millis(1000)).await;
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
                                print_colored(&format!(
                                    "Broker {} received order from Trader {}: {:?} {} shares of {}",
                                    broker_id, order.trader_id, order.order_type, order.quantity, order.stock_symbol
                                ), "cyan");

                                // Serialize the order to JSON
                                let serialized_order = serde_json::to_string(&order).unwrap();
                                // Publish the order to the "orders" queue
                                channel.basic_publish(
                                    "",
                                    "orders",
                                    BasicPublishOptions::default(),
                                    serialized_order.as_bytes(),
                                    BasicProperties::default(),
                                ).await.unwrap();
                                //println!("Order Sent: {}", serialized_order);
                            }
                            None => {
                                print_colored(&format!("Broker {} order channel closed", broker_id), "red");
                                break;
                            }
                        }
                    }
                    status = status_consumer.next() => {
                        match status {
                            Some(Ok(delivery)) => {
                                let status_data = String::from_utf8_lossy(&delivery.data);
                                //println!("Broker {} received order status update: {}", broker_id, status_data);

                                // Deserialize the JSON to order status update data
                                let status_update: OrderStatusUpdate = match serde_json::from_str(&status_data) {
                                    Ok(status_update) => status_update,
                                    Err(err) => {
                                        println!("Broker {} failed to deserialize order status update: {}", broker_id, err);
                                        continue;  // Skip to the next message if deserialization fails
                                    }
                                };

                                // Process the order status update (this can be logged or used to update order states)
                                //println!("Broker {} processing order status update: {:?}", broker_id, status_update);


                                // Update the trader's held stock based on the order status
                                // This is a placeholder for the actual logic to update the trader's portfolio
                                // You need to implement the logic to find the trader and update their portfolio
                                // based on the order status update
                                // Find the trader who made the order
                                let trader_id = &status_update.order_id[5..9];
                                let trader = match trader_id {
                                    "T001" => trader1.clone(),
                                    "T002" => trader2.clone(),
                                    "T003" => trader3.clone(),
                                    _ => {
                                        println!("Broker {} could not find trader with id: {}", broker_id, trader_id);
                                        continue;
                                    }
                                };

                                // Introduce a small delay to ensure the pending order is added
                                //sleep(Duration::from_millis(100)).await;
                                // Complete the order for the trader
                                let mut trader = trader.lock().await;
                                //println!("Pending orders for trader {}: {:?}", trader_id, trader.pending_orders); // Debugging information
                                //println!("Looking for order ID: {}", status_update.order_id); // Debugging information
                                if let Some(pos) = trader.pending_orders.iter().position(|o| o.order_id == status_update.order_id) {
                                    let order = trader.pending_orders.remove(pos);
                                    if let Err(e) = trader.complete_order(&order, stock_prices.get(&order.stock_symbol).cloned().unwrap_or(0.0)) {
                                        println!("Broker {} failed to complete order for trader {}: {}", broker_id, trader_id, e);
                                    } else {
                                        println!("Broker {} successfully completed order for trader {}: {:?}", broker_id, trader_id, order);
                                    }
                                } else {
                                    print_colored(&format!("Broker {} could not find pending order from trader id: {} (trader revert the order) ", broker_id, status_update.order_id), "yellow");
                                    // Clean the pending order with that order ID
                                    trader.remove_pending_order(&status_update.order_id);
                                }

                                // Acknowledge the message
                                delivery.ack(BasicAckOptions::default()).await.unwrap();

                                // Print a success message
                                //print_colored(&format!("Broker {} successfully processed order status update for order_id: {}", broker_id, status_update.order_id), "green");
                            }
                            Some(Err(err)) => {
                                println!("Broker {} failed to receive order status update: {}", broker_id, err);
                            }
                            None => {
                                println!("Broker {} order status consumer closed", broker_id);
                                break;
                            }
                        }
                    }
                }
            }
        });
    }
}