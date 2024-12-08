use amiquip::{ConsumerMessage, ConsumerOptions, QueueDeclareOptions, Result};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Establish connection to RabbitMQ server
    let connection = amiquip::Connection::insecure_open("amqp://guest:guest@localhost:5672")?;
    let channel = connection.open_channel(None)?;
    let queue = channel.queue_declare("order_status", QueueDeclareOptions::default())?;
    let consumer = queue.consume(ConsumerOptions::default())?;

    println!("Order Status Listener: Waiting for order status updates...");

    // Receive messages in a loop
    for message in consumer.receiver().iter() {
        match message {
            ConsumerMessage::Delivery(delivery) => {
                let status_data = String::from_utf8_lossy(&delivery.body);
                println!("Received Order Status Update: {}", status_data);

                // Deserialize the JSON to order status update data
                let status_update: OrderStatusUpdate = match serde_json::from_str(&status_data) {
                    Ok(status_update) => status_update,
                    Err(err) => {
                        println!("Failed to deserialize order status update: {}", err);
                        continue;  // Skip to the next message if deserialization fails
                    }
                };

                // Process the order status update (this can be logged or used to update order states)
                println!(
                    "Order Status Update: Order ID {} - Status: {}",
                    status_update.order.order_id, status_update.order.status
                );

                // Acknowledge the message
                consumer.ack(delivery)?;
            }
            other => {
                println!("Order Status Listener Ended: {:?}", other);
                break;
            }
        }

        // Simulate processing time
        sleep(Duration::from_millis(50)).await;
    }

    connection.close()
}

#[derive(Serialize, Debug, Deserialize)]
struct OrderStatusUpdate {
    broker_id: String,
    trader_id: String,
    order: OrderStatus,
}

#[derive(Serialize, Debug, Deserialize)]
struct OrderStatus {
    order_id: String,
    status: String,
}
