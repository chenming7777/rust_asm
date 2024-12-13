// models.rs

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stock {
    pub symbol: String,
    pub price: f64,
    pub price_change: PriceChange,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PriceChange {
    pub percentage: f64,
    pub absolute: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Order {
    pub trader_id: String,
    pub stock_symbol: String,
    pub order_type: OrderType,
    pub quantity: u32,
    pub limit_price: Option<f64>, // Optional limit price for limit orders
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OrderType {
    MarketBuy,
    LimitBuy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct Trader {
    pub id: String,
    pub cash: f64,
    pub portfolio: Vec<Stock>,
}

impl Trader {
    pub fn new(id: String) -> Self {
        Self {
            id,
            cash: 5000.0, // Default cash amount
            portfolio: Vec::new(),
        }
    }

    pub fn buy_stock(&mut self, stock: Stock, quantity: u32) -> Result<(), String> {
        let total_cost = stock.price * quantity as f64;
        if self.cash >= total_cost {
            self.cash -= total_cost;
            self.portfolio.push(stock);
            Ok(())
        } else {
            Err(format!("Trader {} does not have enough cash to buy {} shares of {}", self.id, quantity, stock.symbol))
        }
    }
}