use amiquip::{Exchange, Publish, Result};
use serde::{Serialize, Deserialize};
use tokio::time::{sleep, Duration};


#[tokio::main]
async fn main() -> Result<(), CustomError> {
    // Establish connection to RabbitMQ server
    let connection = amiquip::Connection::insecure_open("amqp://guest:guest@localhost:5672")?;
    let channel = connection.open_channel(None)?;
    let exchange = Exchange::direct(&channel);

    loop {
        // Create a sample order request
        let order_request = OrderRequest {
            broker_id: "B001".to_string(),
            trader_id: "T001".to_string(),
            order: Order {
                order_id: "ORD000001".to_string(),
                stock_symbol: "AMZN".to_string(),
                order_type: "Limit".to_string(),
                order_action: "Buy".to_string(),
                target_price: 150.00,
                quantity: 5,
            },
        };

        // Serialize the order to JSON
        let serialized_order = serde_json::to_string(&order_request)?;
        // Publish the order to the "orders" queue
        exchange.publish(Publish::new(serialized_order.as_bytes(), "orders"))?;
        println!("Order Sent: {}", serialized_order);

        // Wait for a few seconds before sending the next order
        sleep(Duration::from_secs(5)).await;
    }
}

#[derive(Serialize, Debug, Deserialize)]
struct OrderRequest {
    broker_id: String,
    trader_id: String,
    order: Order,
}

#[derive(Serialize, Debug, Deserialize)]
struct Order {
    order_id: String,
    stock_symbol: String,
    order_type: String,
    order_action: String,
    target_price: f64,
    quantity: usize,
}
