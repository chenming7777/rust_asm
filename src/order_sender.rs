use lapin::{
    options::*, types::FieldTable, BasicProperties, Connection, ConnectionProperties, ExchangeKind,
};
use serde::{Deserialize, Serialize};
use futures::StreamExt; // Import the StreamExt trait
use crate::models::Order; // Import the Order struct

#[derive(Serialize, Debug, Deserialize)]
struct OrderStatusUpdate {
    order_id: String,
    status: String,
}

pub async fn run_order_sender() -> Result<(), Box<dyn std::error::Error>> {
    // Establish connection to RabbitMQ server
    let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;
    // let queue = channel.queue_declare("orders", QueueDeclareOptions::default(), FieldTable::default()).await?;
    let mut consumer = channel.basic_consume("orders", "", BasicConsumeOptions::default(), FieldTable::default()).await?;
    channel.exchange_declare(
        "order_status",
        ExchangeKind::Direct,
        ExchangeDeclareOptions::default(),
        FieldTable::default(),
    ).await?;

    println!("Order Sender: Waiting for orders...");

    // Receive and process orders in a loop
    while let Some(delivery) = consumer.next().await {
        let delivery = delivery?;
        let order_data = String::from_utf8_lossy(&delivery.data);
        //println!("Received Order: {}", order_data);

        // Deserialize the JSON to order data
        let order: Order = match serde_json::from_str(&order_data) {
            Ok(order) => order,
            Err(err) => {
                println!("Failed to deserialize order: {}", err);
                continue;  // Skip to the next message if deserialization fails
            }
        };

        // Process the order (this can be more complex in a real application)
        //println!("Processing Order: {:?}", order);

        // Create an order status update
        let order_status_update = OrderStatusUpdate {
            order_id: order.order_id.clone(),
            status: "complete".to_string(),
        };

        // Serialize the order status update to JSON
        let serialized_status = serde_json::to_string(&order_status_update)?;
        // Publish the order status update to the "order_status" queue
        channel.basic_publish(
            "",
            "order_status",
            BasicPublishOptions::default(),
            serialized_status.as_bytes(),
            BasicProperties::default(),
        ).await?;
        //println!("Order Status Sent: {}", serialized_status);

        // Acknowledge the message
        delivery.ack(BasicAckOptions::default()).await?;
    }

    Ok(())
}