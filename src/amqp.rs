use futures_lite::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};

use tokio::sync::mpsc;
use tracing::info;

use crate::payload::Payload;

const Q: &str = "robserver.messages";
const CONSUMER_TAG: &str = "robserver.ct";

pub async fn listen_messages(tx: mpsc::Sender<Payload>) {
	let addr = std::env::var("ROBSERVER_AMQP_ADDR")
		.unwrap_or_else(|_| "amqp://guest:guest@127.0.0.1:5672/%2f".into());

	let exchanges = std::env::var("ROBSERVER_LISTEN_EX")
		.unwrap_or_else(|_| "amq.direct,amq.fanout,amq.headers,amq.topic".into());
	let exchanges = exchanges.split(',');

	let conn = Connection::connect(&addr, ConnectionProperties::default())
		.await
		.expect("connection error");

	info!("CONNECTED");

	//receive channel
	let channel = conn.create_channel().await.expect("create_channel");
	info!(state=?conn.status().state());

	let queue = channel
		.queue_declare(Q, QueueDeclareOptions::default(), FieldTable::default())
		.await
		.expect("queue_declare");
	info!(state=?conn.status().state());
	info!(?queue, "Declared queue");

	for ex in exchanges.into_iter() {
		info!(exchange = ex, "Setting up binding");
		channel
			.queue_bind(
				Q,
				ex,
				ex,
				QueueBindOptions::default(),
				FieldTable::default(),
			)
			.await
			.expect("queue_bind");
	}

	channel.basic_qos(10, BasicQosOptions::default());

	info!("will consume");
	let mut consumer = channel
		.basic_consume(
			Q,
			CONSUMER_TAG,
			BasicConsumeOptions::default(),
			FieldTable::default(),
		)
		.await
		.expect("basic_consume");
	info!(state=?conn.status().state());

	while let Some(delivery) = consumer.next().await {
		let message = delivery.unwrap();

		let payload = Payload::new(message.data, "/".to_string(), message.exchange.to_string());

		tx.send(payload).await.expect("tx send");

		tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

		message
			.acker
			.nack(BasicNackOptions {
				multiple: false,
				requeue: true,
			})
			.await
			.expect("ack");
		// if let Ok(delivery) = delivery {		}
	}
}
