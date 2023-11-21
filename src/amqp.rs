use futures_lite::StreamExt;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Queue};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::config::amqp as config;
use crate::payload::Payload;

const Q: &str = "robserver.messages";
const CONSUMER_TAG: &str = "robserver.ct";

pub async fn declare_queue(channel: &Channel) -> Result<Queue, lapin::Error> {
	let mut fields = FieldTable::default();
	fields.insert("x-max-length".into(), config::get_queue_max_length().into());

	let options = QueueDeclareOptions {
		durable: false,
		exclusive: false,
		auto_delete: true,
		..QueueDeclareOptions::default()
	};

	channel.queue_declare(Q, options, fields).await
}

#[allow(dead_code)]
pub async fn retry_declare_queue(channel: &Channel) -> Queue {
	let mut result = declare_queue(channel).await;

	while let Err(err) = result {
		error!("error declaring queue: {}", err);

		result = declare_queue(channel).await
	}

	result.unwrap()
}

pub async fn listen_messages(tx: mpsc::Sender<Payload>) {
	let addr = config::get_url();
	let exchanges = config::get_exchanges();
	let prefetch = config::get_prefetch();

	let conn = Connection::connect(&addr, ConnectionProperties::default())
		.await
		.expect("connection error");

	info!("CONNECTED");

	//receive channel
	let channel = conn.create_channel().await.expect("create_channel");
	info!(state=?conn.status().state());

	let queue = declare_queue(&channel).await;

	info!(state=?conn.status().state());
	info!(?queue, "Declared queue");

	{
		let channel = conn.create_channel().await.expect("create_channel");

		for ex in exchanges {
			info!(exchange = ex, "Setting up binding");
			let _ = channel
				.queue_bind(
					Q,
					ex.as_str(),
					"#",
					QueueBindOptions::default(),
					FieldTable::default(),
				)
				.await
				.is_err_and(|err| {
					error!(error = err.to_string(), "Failed to bind robserver to queue");
					false
				});
		}
	}

	let res = channel
		.basic_qos(prefetch, BasicQosOptions::default())
		.await;
	info!("set prefetch to {}: {:?}", prefetch, res);

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

		message
			.acker
			.ack(BasicAckOptions { multiple: true })
			.await
			.expect("ack");
	}
}
