use futures_lite::StreamExt;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Queue};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tracing::info;

use crate::config::amqp as config;
use crate::payload::Payload;

const CONSUMER_TAG: &str = "robserver.ct";

pub async fn declare_work_queue(channel: &Channel, queue_name: &String) -> Result<Queue, lapin::Error> {
	let mut fields = FieldTable::default();
	fields.insert("x-max-length".into(), config::get_queue_max_length().into());

	let options = QueueDeclareOptions {
		durable: false,
		exclusive: false,
		auto_delete: true,
		..QueueDeclareOptions::default()
	};

	channel.queue_declare(queue_name, options, fields).await
}

pub async fn listen_messages(tx: mpsc::Sender<Payload>) {
	info!("Connecting...");
	let addr = config::get_url();
	let exchanges = config::get_exchanges();
	let prefetch = config::get_prefetch();
	let work_queue = config::get_queue();

	let conn = timeout(Duration::from_secs(5), async {
		let conn = Connection::connect(&addr, ConnectionProperties::default())
			.await
			.expect("Failed to connect to RabbitMQ");

		return conn;
	})
	.await
	.expect("Failed to connect to RabbitMQ");

	info!("Connected");

	let channel = conn.create_channel().await.expect("create_channel");
	let queue = declare_work_queue(&channel, &work_queue).await;
	info!(?queue, "Declared queue");

	if exchanges.len() == 0 {
		info!("No exchanges to bind to");
	} else {
		let mut channel = conn.create_channel().await.expect("create_channel");

		for ex in exchanges {
			match channel
				.queue_bind(
					&work_queue,
					ex.as_str(),
					"#",
					QueueBindOptions::default(),
					FieldTable::default(),
				)
				.await
			{
				Ok(_) => {
					info!(exchange = ex, "Successfully bound")
				}
				Err(_) => {
					channel = conn.create_channel().await.unwrap();
				}
			};
		}
	}

	channel
		.basic_qos(prefetch, BasicQosOptions::default())
		.await
		.expect("Failed to set prefetch");

	channel.on_error(|_| {
		info!("channel error");
	});

	info!("Consuming");
	let mut consumer = channel
		.basic_consume(
			&work_queue,
			CONSUMER_TAG,
			BasicConsumeOptions::default(),
			FieldTable::default(),
		)
		.await
		.expect("Failed to consume");

	while let Some(delivery) = consumer.next().await {
		let message = delivery.unwrap();

		let payload = Payload::new(message.data, "/".to_string(), message.exchange.to_string());

		tx.send(payload)
			.await
			.expect("Could not send payload for processing");

		message
			.acker
			.ack(BasicAckOptions { multiple: true })
			.await
			.expect("Failed to ack");
	}
}
