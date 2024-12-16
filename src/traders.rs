use tokio::sync::mpsc;
use tokio::sync::Mutex;
use crate::models::{Stock, Order, OrderType, PriceChange}; // Import the PriceChange struct
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;
use crate::color::print_colored; // Import the print_colored function

#[derive(Debug, Clone)]
pub struct Trader {
    pub id: String,
    pub cash: f64,
    pub portfolio: Vec<Stock>,
    pub pending_orders: Vec<Order>,
}

impl Trader {
    pub fn new(id: String) -> Self {
        Self {
            id,
            cash: 5000.0, // Default cash amount
            portfolio: Vec::new(),
            pending_orders: Vec::new(),
        }
    }

    pub fn buy_stock(&mut self, stock: Stock, quantity: u32) -> Result<(), String> {
        let total_cost = stock.price * quantity as f64;
        if self.cash >= total_cost {
            self.cash -= total_cost;
            let mut found = false;
            for held_stock in &mut self.portfolio {
                if held_stock.symbol == stock.symbol {
                    let total_quantity = held_stock.price * held_stock.price_change.absolute + stock.price * quantity as f64;
                    let new_quantity = held_stock.price_change.absolute + quantity as f64;
                    held_stock.price = total_quantity / new_quantity;
                    held_stock.price_change.absolute = new_quantity;
                    found = true;
                    break;
                }
            }
            if !found {
                let mut new_stock = stock.clone();
                new_stock.price_change.absolute = quantity as f64;
                self.portfolio.push(new_stock);
            }
            Ok(())
        } else {
            Err(format!("Trader {} does not have enough cash to buy {} shares of {}", self.id, quantity, stock.symbol))
        }
    }

    pub fn sell_stock(&mut self, stock_symbol: &str, quantity: u32) -> Result<f64, String> {
        let mut total_revenue = 0.0;
        let mut sold_quantity = 0;

        self.portfolio.retain(|stock| {
            if stock.symbol == stock_symbol && sold_quantity < quantity {
                total_revenue += stock.price;
                sold_quantity += 1;
                false
            } else {
                true
            }
        });

        if sold_quantity == quantity {
            self.cash += total_revenue;
            Ok(total_revenue)
        } else {
            Err(format!("Trader {} does not have {} shares of {}", self.id, quantity, stock_symbol))
        }
    }

    pub fn add_pending_order(&mut self, order: Order) {
        self.pending_orders.push(order);
    }

    pub fn complete_order(&mut self, order: &Order, stock_price: f64) -> Result<(), String> {
        match order.order_type {
            OrderType::MarketBuy | OrderType::LimitBuy => {
                let stock = Stock {
                    symbol: order.stock_symbol.clone(),
                    price: stock_price,
                    price_change: PriceChange { percentage: 0.0, absolute: 0.0 },
                };
                self.buy_stock(stock, order.quantity)
            }
            OrderType::MarketSell | OrderType::LimitSell => {
                self.sell_stock(&order.stock_symbol, order.quantity).map(|_| ())
            }
        }
    }

    pub fn remove_pending_order(&mut self, order: &Order) {
        self.pending_orders.retain(|o| o != order);
    }

    pub fn cancel_pending_orders(&mut self) {
        for order in &self.pending_orders {
            match order.order_type {
                OrderType::MarketBuy | OrderType::LimitBuy => {
                    let total_cost = order.limit_price.unwrap_or(0.0) * order.quantity as f64;
                    self.cash += total_cost;
                }
                OrderType::MarketSell | OrderType::LimitSell => {
                    // No action needed for sell orders
                }
            }
        }
        self.pending_orders.clear();
    }
}

// Trader function: Listens to its broker's updates and processes stock data
pub async fn run_trader(
    trader_id: String,
    mut stock_rx: mpsc::Receiver<Stock>,
    order_tx: mpsc::Sender<Order>,
    trader: Arc<Mutex<Trader>>, // Pass the trader as an Arc<Mutex<Trader>>
) {
    println!("Trader {} started.", trader_id);
    let rng = Arc::new(Mutex::new(StdRng::from_entropy()));

    loop {
        match stock_rx.recv().await {
            Some(stock) => {
                println!(
                    "Trader {} received stock update: Symbol: {}, Price: ${:.2}",
                    trader_id, stock.symbol, stock.price
                );
                // Simulate decision-making (e.g., market buy/limit buy/hold/sell)
                let mut rng = rng.lock().await;
                let decision = rng.gen_range(0..=3);
                match decision {
                    0 => {
                        // Market buy
                        print_colored(
                            &format!("Trader {} decided to market buy stock: {}", trader_id, stock.symbol),
                            "green"
                        );
                        let quantity = rng.gen_range(1..=10);
                        let order = Order {
                            trader_id: trader_id.clone(),
                            stock_symbol: stock.symbol.clone(),
                            order_type: OrderType::MarketBuy,
                            quantity,
                            limit_price: None,
                        };
                        // Add pending order
                        let mut trader = trader.lock().await;
                        trader.add_pending_order(order.clone());
                        // Send the order to the broker
                        if let Err(e) = order_tx.send(order).await {
                            print_colored(&format!("Trader {} failed to send order: {:?}", trader_id, e), "red");
                        }
                    }
                    1 => {
                        // Limit buy
                        let limit_price = rng.gen_range(stock.price * 0.9..=stock.price * 1.1);
                        print_colored(
                            &format!("Trader {} decided to limit buy stock: {} at price ${:.2}", trader_id, stock.symbol, limit_price),
                            "green"
                        );
                        let quantity = rng.gen_range(1..=10);
                        let order = Order {
                            trader_id: trader_id.clone(),
                            stock_symbol: stock.symbol.clone(),
                            order_type: OrderType::LimitBuy,
                            quantity,
                            limit_price: Some(limit_price),
                        };
                        // Add pending order
                        let mut trader = trader.lock().await;
                        trader.add_pending_order(order.clone());
                        // Send the order to the broker
                        if let Err(e) = order_tx.send(order).await {
                            print_colored(&format!("Trader {} failed to send order: {:?}", trader_id, e), "red");
                        }
                    }
                    2 => {
                        // Market sell
                        let mut trader = trader.lock().await;
                        let quantity = rng.gen_range(1..=10);
                        match trader.sell_stock(&stock.symbol, quantity) {
                            Ok(revenue) => {
                                print_colored(
                                    &format!("Trader {} decided to market sell {} shares of {} for ${:.2}", trader_id, quantity, stock.symbol, revenue),
                                    "red"
                                );
                                let order = Order {
                                    trader_id: trader_id.clone(),
                                    stock_symbol: stock.symbol.clone(),
                                    order_type: OrderType::MarketSell,
                                    quantity,
                                    limit_price: None,
                                };
                                // Add pending order
                                trader.add_pending_order(order.clone());
                                // Send the order to the broker
                                if let Err(e) = order_tx.send(order).await {
                                    print_colored(&format!("Trader {} failed to send order: {:?}", trader_id, e), "red");
                                }
                            }
                            Err(err) => print_colored(&err, "red"),
                        }
                    }
                    _ => {
                        // Hold
                        print_colored(&format!("Trader {} decided to hold on stock: {}", trader_id, stock.symbol), "yellow");
                    }
                }
            }
            None => {
                print_colored(&format!("Trader {} channel closed", trader_id), "red");
                break;
            }
        }
    }
}