use futures_lite::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties, Channel, Queue};
use tokio::sync::mpsc;
use tracing::{info, error};

use crate::payload::Payload;
use crate::config::amqp as config;

const Q: &str = "robserver.messages";
const CONSUMER_TAG: &str = "robserver.ct";

pub async fn declare_queue(channel: &Channel) -> Result<Queue, lapin::Error> {
	let mut fields = FieldTable::default();
	fields.insert("x-max-length".into(), 10_000.into());

	let options = QueueDeclareOptions {
		durable: false,
		exclusive: false,
		auto_delete: true,
		..QueueDeclareOptions::default()
	};

	channel
		.queue_declare(Q, options, fields)
		.await
}

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
			channel
				.queue_bind(
					Q,
					ex.as_str(),
					"#",
					QueueBindOptions::default(),
					FieldTable::default(),
				)
				.await
				.is_err_and(|err| {
					error!(error=err.to_string(), "Failed to bind robserver to queue");
					false
				});
		}
	}


	channel.basic_qos(prefetch, BasicQosOptions::default());

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

		// tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

		message
			.acker
			.ack(BasicAckOptions {
				multiple: true,
			})
			.await
			.expect("ack");
	}
}
