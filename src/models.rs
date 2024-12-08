use serde::{Deserialize, Serialize};

// Struct to represent a Stock
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct Stock {
    pub symbol: String,
    pub price: f64,
    pub price_change: PriceChange,
}

// Struct to represent the PriceChange
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct PriceChange {
    pub percentage: f64,
    pub absolute: f64,
}
