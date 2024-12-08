use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use crate::models::Stock;

// Broker function: Handles stock updates and broadcasts them to traders
pub async fn run_brokers(tx: broadcast::Sender<Stock>) {
    for i in 1..=5 {
        let broker_id = format!("B{:03}", i);
        let mut stock_rx = tx.subscribe();

        tokio::spawn(async move {
            println!("Broker {} started.", broker_id);

            loop {
                match stock_rx.recv().await {
                    Ok(stock) => {
                        println!(
                            "Broker {} received stock: Symbol: {}, Price: ${:.2}",
                            broker_id, stock.symbol, stock.price
                        );

                        // Simulate broadcasting to traders
                        sleep(Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        println!("Broker {} error receiving stock update: {}", broker_id, e);
                        break;
                    }
                }
            }
        });
    }
}
