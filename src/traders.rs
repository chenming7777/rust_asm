use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use crate::models::Stock;
use rand::Rng;

// Trader function: Listens to its broker's updates and processes stock data
pub async fn run_trader(trader_id: String, mut stock_rx: broadcast::Receiver<Stock>) {
    println!("Trader {} started.", trader_id);

    loop {
        match stock_rx.recv().await {
            Ok(stock) => {
                println!(
                    "Trader {} received stock update: Symbol: {}, Price: ${:.2}",
                    trader_id, stock.symbol, stock.price
                );

                // Simulate decision-making (e.g., buy/sell/hold)
                let mut rng = rand::thread_rng();
                if rng.gen_bool(0.5) {
                    println!(
                        "Trader {} decided to take action on stock: {}",
                        trader_id, stock.symbol
                    );
                } else {
                    println!("Trader {} decided to hold on stock: {}", trader_id, stock.symbol);
                }
            }
            Err(e) => {
                println!("Trader {} error receiving stock update: {}", trader_id, e);
                break;
            }
        }

        // Simulate a small delay for processing
        sleep(Duration::from_millis(200)).await;
    }
}
