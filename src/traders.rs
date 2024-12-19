use tokio::sync::mpsc;
use tokio::sync::Mutex;
use crate::models::{Stock, Order, OrderType, PriceChange};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;
use crate::color::print_colored; // Import the print_colored function

#[derive(Debug, Clone)]
pub struct OwnedPosition {
    pub symbol: String,
    pub quantity: u32,
    pub average_cost: f64,
}

#[derive(Debug, Clone)]
pub struct Trader {
    pub id: String,
    pub cash: f64,
    pub portfolio: Vec<OwnedPosition>,
    pub pending_orders: Vec<Order>,
    pub order_counter: u64, // Counter for generating unique order IDs
}


impl Trader {
    pub fn new(id: String) -> Self {
        Self {
            id,
            cash: 5000.0, // Default cash amount
            portfolio: Vec::new(),
            pending_orders: Vec::new(),
            order_counter: 0, // Initialize the order counter
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
                self.buy_stock(stock, order.quantity)?;
                print_colored(&format!("Trader {} bought {} shares of {} at ${:.2} each.", self.id, order.quantity, order.stock_symbol, stock_price), "green");
            }
            OrderType::MarketSell | OrderType::LimitSell => {
                self.sell_stock(&order.stock_symbol, order.quantity)?;
                print_colored(&format!("Trader {} sold {} shares of {} at ${:.2} each.", self.id, order.quantity, order.stock_symbol, stock_price), "red");
            }
        }
        // Remove the pending order after processing
        self.remove_pending_order(&order.order_id);
        Ok(())
    }

    pub fn remove_pending_order(&mut self, order_id: &str) {
        self.pending_orders.retain(|o| o.order_id != order_id);
    }

    pub fn cancel_pending_orders(&mut self) {
        let mut rng = rand::thread_rng();
        let variation = rng.gen_range(0.95..=1.01);
        let average_cost = 100.0 * variation;
        for order in &self.pending_orders {
            match order.order_type {
                OrderType::MarketBuy | OrderType::LimitBuy => {
                    let total_cost = order.limit_price.unwrap_or(average_cost) * order.quantity as f64;
                    self.cash += total_cost;
                }
                OrderType::MarketSell | OrderType::LimitSell => {
                   // Return the quantity of the stock back to the trader's portfolio
                   if let Some(position) = self.portfolio.iter_mut().find(|p| p.symbol == order.stock_symbol) {
                    position.quantity += order.quantity;
                } else {
                    self.portfolio.push(OwnedPosition {
                        symbol: order.stock_symbol.clone(),
                        quantity: order.quantity,
                        average_cost,
                    });
                }
                }
            }
        }
        self.pending_orders.clear();
    }

    fn generate_order_id(&mut self) -> String {
        self.order_counter += 1;
        format!("{}-{}", self.id, self.order_counter)
    }

    // This function happen on after buying stock
    // This function is to buy new stock existing stock will find the average cost of all quantity 
    // to the stock whereas new stock will be added to the portfolio
    pub fn buy_stock(&mut self, stock: Stock, quantity: u32) -> Result<(), String> {
        let total_cost = stock.price * quantity as f64;
        if self.cash >= total_cost {
            self.cash -= total_cost;
            let mut found = false;
            for held_stock in &mut self.portfolio {
                if held_stock.symbol == stock.symbol {
                    let total_quantity = held_stock.quantity + quantity;
                    held_stock.average_cost = (held_stock.average_cost * held_stock.quantity as f64 + stock.price * quantity as f64) / total_quantity as f64;
                    held_stock.quantity = total_quantity;
                    found = true;
                    break;
                }
            }
            if !found {
                let new_stock = OwnedPosition {
                    symbol: stock.symbol.clone(),
                    quantity,
                    average_cost: stock.price,
                };
                self.portfolio.push(new_stock);
            }
            Ok(())
        } else {
            Err(format!("Trader {} only have {} cash, cannot buy {} shares of {}", self.id, self.cash, quantity, stock.symbol))
        }
    }

    pub fn sell_stock(&mut self, stock_symbol: &str, quantity: u32) -> Result<f64, String> {
        let mut found = false;
        let mut total_revenue = 0.0;
        for held_stock in &mut self.portfolio {
            if held_stock.symbol == stock_symbol {
                if held_stock.quantity >= quantity {
                    total_revenue = held_stock.average_cost * quantity as f64;
                    held_stock.quantity -= quantity;
                    self.cash += total_revenue;
                    found = true;
                    break;
                } else {
                    return Err(format!("Trader {} does not have enough shares of {}", self.id, stock_symbol));
                }
            }
        }
        if found {
            self.portfolio.retain(|stock| stock.quantity > 0);
            Ok(total_revenue)
        } else {
            Err(format!("Trader {} does not own any shares of {}", self.id, stock_symbol))
        }
    }
}


fn determine_decision(price_change: f64, rng: &mut StdRng) -> Option<u8> {
    // Default probabilities
    let mut buy_probability = 0.001;
    let mut sell_probability = 0.001;

    // Adjust probabilities based on price change
    if price_change > 0.0 {
        // Increase sell probability and decrease buy probability if price is rising
        sell_probability += price_change / 10000.0;
        buy_probability -= price_change / 10000.0;
    } else {
        // Increase buy probability and decrease sell probability if price is falling
        buy_probability += price_change.abs() / 10000.0;
        sell_probability -= price_change.abs() / 10000.0;
    }

    // Generate a random decision based on adjusted probabilities
    let decision_probability: f64 = rng.gen_range(0.0..1.0);
    if decision_probability < buy_probability {
        // Buy decision
        Some(rng.gen_range(0..=1)) // Market buy or limit buy
    } else if decision_probability < buy_probability + sell_probability {
    // } else if decision_probability < 0.0000001 {

        // Sell decision
        Some(rng.gen_range(2..=3)) // Market sell or limit sell
    } else {
        // Hold decision
        None
    }
}


pub async fn run_trader(
    trader_id: String,
    mut stock_rx: mpsc::Receiver<Stock>,
    order_tx: mpsc::Sender<Order>,
    trader: Arc<Mutex<Trader>>, // Pass the trader as an Arc<Mutex<Trader>>
) {
    println!("Trader {} ready to trade.", trader_id);
    // This is for any random number generation
    let rng = Arc::new(Mutex::new(StdRng::from_entropy()));

    loop {
        match stock_rx.recv().await {
            Some(stock) => {
                // println!(
                //     "Trader {} received stock update: Symbol: {}, Price: ${:.2}",
                //     trader_id, stock.symbol, stock.price
                // );
                
                
                // Simulate decision-making (e.g., market buy/limit buy/hold/market sell/limit sell)
                let mut rng = rng.lock().await;
                if let Some(decision) = determine_decision(stock.price_change.percentage, &mut rng) {
                    match decision {
                        0 => {
                            // Market buy
                            print_colored(
                                &format!("Trader {} decided to buy stock: {} on {}", trader_id, stock.symbol, stock.price),
                                "blue"
                            );
                            let quantity = rng.gen_range(1..=5);
                            let total_cost = stock.price * quantity as f64;
                            let mut trader = trader.lock().await;
                            if trader.cash >= total_cost {
                                let order_id = trader.generate_order_id();
                                let order = Order {
                                    order_id,
                                    trader_id: trader_id.clone(),
                                    stock_symbol: stock.symbol.clone(),
                                    order_type: OrderType::MarketBuy,
                                    quantity,
                                    limit_price: None,
                                };
                                // Add pending order
                                trader.add_pending_order(order.clone());
                                // Deduct cash
                                trader.cash -= total_cost;
                                print_colored(
                                    &format!("Trader {} sent market order to buy stock: {} on {}", trader_id, stock.symbol, stock.price),
                                    "green"
                                );
                                // Send the order to the broker
                                if let Err(e) = order_tx.send(order).await {
                                    print_colored(&format!("Trader {} failed to send order: {:?}", trader_id, e), "red");
                                }
                            } else {
                                print_colored(&format!("Trader {} does not have enough cash to buy {} shares of {}", trader_id, quantity, stock.symbol), "red");
                            }
                        }
                        1 => {
                            // Limit buy
                            let limit_price = rng.gen_range(stock.price * 0.95..=stock.price);
                            print_colored(
                                &format!("Trader {} decided to limit buy stock: {} at price ${:.2}", trader_id, stock.symbol, limit_price),
                                "green"
                            );
                            let quantity = rng.gen_range(1..=5);
                            let total_cost = limit_price * quantity as f64;
                            let mut trader = trader.lock().await;
                            if trader.cash >= total_cost {
                                let order_id = trader.generate_order_id();
                                let order = Order {
                                    order_id:order_id.clone(),
                                    trader_id: trader_id.clone(),
                                    stock_symbol: stock.symbol.clone(),
                                    order_type: OrderType::LimitBuy,
                                    quantity,
                                    limit_price: Some(limit_price),
                                };
                                // Add pending order
                                trader.add_pending_order(order.clone());
                                // Deduct cash
                                trader.cash -= total_cost;
                                // Send the order to the broker
                                if let Err(e) = order_tx.send(order).await {
                                    print_colored(&format!("Trader {} failed to send order: {:?}", trader_id, e), "red");
                                    // If sending the order fails, remove it from pending orders
                                    trader.remove_pending_order(&order_id);
                                }
                            } else {
                                print_colored(&format!("Trader {} does not have enough cash to buy {} shares of {}", trader_id, quantity, stock.symbol), "red");
                            }
                        }
                        2 => {
                            // Market sell
                            let mut trader = trader.lock().await;
                            let quantity = rng.gen_range(1..=5);
                            let order_id = trader.generate_order_id();
                            let order = Order {
                                order_id: order_id.clone(),
                                trader_id: trader_id.clone(),
                                stock_symbol: stock.symbol.clone(),
                                order_type: OrderType::MarketSell,
                                quantity,
                                limit_price: None,
                            };
                            // Add pending order
                            trader.add_pending_order(order.clone());
                            print_colored(
                                &format!("Trader {} decided to market sell {} shares of {}", trader_id, quantity, stock.symbol),
                                "red"
                            );
                            // Send the order to the broker
                            if let Err(e) = order_tx.send(order).await {
                                print_colored(&format!("Trader {} failed to send order: {:?}", trader_id, e), "red");
                                // If sending the order fails, remove it from pending orders
                                trader.remove_pending_order(&order_id);
                            }
                        }
                        3 => {
                            // Limit sell
                            let limit_price = rng.gen_range(stock.price..=stock.price * 1.05);
                            print_colored(
                                &format!("Trader {} decided to limit sell stock: {} at price ${:.2}", trader_id, stock.symbol, limit_price),
                                "red"
                            );
                            let quantity = rng.gen_range(1..=10);
                            let mut trader = trader.lock().await;
                            let order_id = trader.generate_order_id();
                            let order = Order {
                                order_id: order_id.clone(),
                                trader_id: trader_id.clone(),
                                stock_symbol: stock.symbol.clone(),
                                order_type: OrderType::LimitSell,
                                quantity,
                                limit_price: Some(limit_price),
                            };
                            // Add pending order
                            trader.add_pending_order(order.clone());
                            // Send the order to the broker
                            if let Err(e) = order_tx.send(order).await {
                                print_colored(&format!("Trader {} failed to send order: {:?}", trader_id, e), "red");
                                // If sending the order fails, remove it from pending orders
                                trader.remove_pending_order(&order_id);
                            }
                        }
                        _ => {}
                    }
                } else {
                    // Hold decision
                    // print_colored(
                    //     &format!("Trader {} decided to hold stock: {}", trader_id, stock.symbol),
                    //     "yellow"
                    // );
                }
            }
            None => {
                println!("Trader {} stock channel closed.", trader_id);
                break;
            }
        }
    }
}