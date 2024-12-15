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
    MarketSell,
    LimitSell,
}