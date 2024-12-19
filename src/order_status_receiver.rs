use lapin::{
    options::*, types::FieldTable, BasicProperties, Connection, ConnectionProperties, ExchangeKind,
};
use serde::{Deserialize, Serialize};
use futures::StreamExt; // Import the StreamExt trait

#[derive(Serialize, Debug, Deserialize)]
struct OrderStatusUpdate {
    order_id: String,
    status: String,
}

pub async fn run_order_status_receiver() -> Result<(), Box<dyn std::error::Error>> {
    // Establish connection to RabbitMQ server for receiving order status updates
    let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;
    //let queue = channel.queue_declare("order_status", QueueDeclareOptions::default(), FieldTable::default()).await?;
    let mut consumer = channel.basic_consume("order_status", "", BasicConsumeOptions::default(), FieldTable::default()).await?;

    // Establish connection to RabbitMQ server for sending processed status updates
    let send_conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default()).await?;
    let send_channel = send_conn.create_channel().await?;
    send_channel.exchange_declare(
        "processed_order_status",
        ExchangeKind::Direct,
        ExchangeDeclareOptions::default(),
        FieldTable::default(),
    ).await?;

    //println!("Order Status Listener: Waiting for order status updates...");

    // Receive messages in a loop
    while let Some(delivery) = consumer.next().await {
        let delivery = delivery?;
        let status_data = String::from_utf8_lossy(&delivery.data);
        //println!("Received Order Status Update: {}", status_data);

        // Deserialize the JSON to order status update data
        let status_update: OrderStatusUpdate = match serde_json::from_str(&status_data) {
            Ok(status_update) => status_update,
            Err(err) => {
                println!("Failed to deserialize order status update: {}", err);
                continue;  // Skip to the next message if deserialization fails
            }
        };

        // Process the order status update (this can be logged or used to update order states)
        //println!("Processing Order Status Update: {:?}", status_update);


        // Serialize the processed order status update to JSON
        let serialized_status = serde_json::to_string(&status_update)?;

        // Publish the processed order status update to the "processed_order_status" queue
        send_channel.basic_publish(
            "",
            "processed_order_status",
            BasicPublishOptions::default(),
            serialized_status.as_bytes(),
            BasicProperties::default(),
        ).await?;
        //println!("Processed Order Status Sent: {}", serialized_status);

        // Acknowledge the message
        delivery.ack(BasicAckOptions::default()).await?;
    }

    Ok(())
}