use crate::traders::Trader;
use crate::models::Order;
use crate::stock_listener::StockStore;

#[derive(Debug)]
pub struct Portfolio {
    pub trader_id: String,
    pub cash_left: f64,
    pub held_stocks: Vec<(String, f64, f64, u32)>, // (stock symbol, latest price, average cost price, quantity)
    pub total_amount: f64,
    pub profit_loss: f64,
    pub pending_orders: Vec<Order>, // Add pending orders
}

impl Portfolio {
    pub async fn new(trader: &Trader, stock_store: &StockStore) -> Self {
        let mut held_stocks = Vec::new();
        let mut total_stock_value = 0.0;

        let store = stock_store.read().await;

        for stock in &trader.portfolio {
            let latest_price = store.get(&stock.symbol).map(|s| s.price).unwrap_or(stock.average_cost);
            held_stocks.push((stock.symbol.clone(), latest_price, stock.average_cost, stock.quantity));
            total_stock_value += latest_price * stock.quantity as f64;
        }

        let total_amount = trader.cash + total_stock_value;

        Self {
            trader_id: trader.id.clone(),
            cash_left: trader.cash,
            held_stocks,
            total_amount,
            profit_loss: total_amount - 5000.0, // Assuming initial cash was 5000.0
            pending_orders: trader.pending_orders.clone(), // Include pending orders
        }
    }

    pub fn display(&self) {
        println!("Trader ID: {}", self.trader_id);
        println!("Cash Left: ${:.2}", self.cash_left);
        println!("Held Stocks:");
        for stock in &self.held_stocks {
            println!(
                "  Symbol: {}, Latest Price: ${:.2}, Average Cost: ${:.2}, Quantity: {}",
                stock.0, stock.1, stock.2, stock.3
            );
        }
        println!("Total Amount: ${:.2}", self.total_amount);
        println!("Profit/Loss: ${:.2}", self.profit_loss);
        println!("Pending Orders: {:?}", self.pending_orders); // Display pending orders
    }
}

pub async fn display_all_portfolios(traders: &[Trader], stock_store: &StockStore) {
    for trader in traders {
        let portfolio = Portfolio::new(trader, stock_store).await;
        portfolio.display();
        println!("-----------------------------");
    }
}