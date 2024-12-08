use amiquip::{Connection, ConsumerMessage, ConsumerOptions, QueueDeclareOptions, Result};
use tokio::sync::broadcast;
use crate::models::Stock; // Import the Stock struct from models.rs

// Function to run the stock listener
pub async fn run_stock_listener(tx: broadcast::Sender<Stock>) -> Result<()> {
    // Open a connection to RabbitMQ server
    let mut connection = Connection::insecure_open("amqp://guest:guest@localhost:5672")?;

    // Open a channel - None says let the library choose the channel ID.
    let channel = connection.open_channel(None)?;

    // Declare the "stocks" queue where the messages will be received.
    let queue = channel.queue_declare("stocks", QueueDeclareOptions::default())?;
    let consumer = queue.consume(ConsumerOptions::default())?;
    println!("Stock Listener: Waiting for stock messages. Press Ctrl-C to exit.");

    // Receive messages in a loop
    for message in consumer.receiver().iter() {
        match message {
            ConsumerMessage::Delivery(delivery) => {
                let body = String::from_utf8_lossy(&delivery.body);
                println!("Received message: {}", body);

                // Deserialize the JSON to stock data to verify correctness
                let stocks: Vec<Stock> = match serde_json::from_str(&body) {
                    Ok(stocks) => stocks,
                    Err(err) => {
                        println!("Failed to deserialize message: {}", err);
                        continue;  // Skip to the next message if deserialization fails
                    }
                };

                // Broadcast each individual stock to all brokers
                for stock in stocks {
                    if let Err(e) = tx.send(stock.clone()) {
                        println!("Failed to broadcast stock data: {}", e);
                    }
                }

                // Acknowledge the message
                consumer.ack(delivery)?;
            }
            other => {
                println!("Consumer ended: {:?}", other);
                break;
            }
        }
    }

    // Close the connection (this line will be reached if the loop ends)
    connection.close()
}
