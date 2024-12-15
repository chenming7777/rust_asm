use crate::traders::Trader;

#[derive(Debug)]
pub struct Portfolio {
    pub trader_id: String,
    pub cash_left: f64,
    pub held_stocks: Vec<(String, f64, u32)>, // (stock symbol, price, quantity)
    pub total_amount: f64,
    pub profit_loss: f64,
}

impl Portfolio {
    pub fn new(trader: &Trader) -> Self {
        let mut held_stocks = Vec::new();
        let mut total_stock_value = 0.0;

        for stock in &trader.portfolio {
            let quantity = held_stocks.iter_mut().find(|(symbol, _, _)| symbol == &stock.symbol);
            if let Some((_, price, qty)) = quantity {
                *qty += 1;
                *price = stock.price; // Update to the latest price
            } else {
                held_stocks.push((stock.symbol.clone(), stock.price, 1));
            }
            total_stock_value += stock.price;
        }

        let total_amount = trader.cash + total_stock_value;

        Self {
            trader_id: trader.id.clone(),
            cash_left: trader.cash,
            held_stocks,
            total_amount,
            profit_loss: total_amount - 5000.0, // Assuming initial cash was 5000.0
        }
    }

    pub fn display(&self) {
        println!("Trader ID: {}", self.trader_id);
        println!("Cash Left: ${:.2}", self.cash_left);
        println!("Held Stocks: {:?}", self.held_stocks);
        println!("Total Amount: ${:.2}", self.total_amount);
        println!("Profit/Loss: ${:.2}", self.profit_loss);
    }
}

pub fn display_all_portfolios(traders: &[Trader]) {
    for trader in traders {
        let portfolio = Portfolio::new(trader);
        portfolio.display();
        println!("-----------------------------");
    }
}