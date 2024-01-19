use futures_lite::StreamExt;
use lapin::{options::*, types::FieldTable, Channel};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::config::amqp as config;
use crate::payload::Payload;

use super::CONSUMER_TAG;
use super::VHOST;

pub async fn payload_parser(payloads: mpsc::Sender<Payload>, channel: Channel) {
	let prefetch = config::get_prefetch();
	let work_queue = config::get_queue();

	channel
		.basic_qos(prefetch, BasicQosOptions::default())
		.await
		.expect("Failed to set prefetch");

	channel.on_error(|error| {
		error!(?error, "Channel error");
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
		debug!(?message, "Message recieved");

		let payload = Payload::new(
			message.data,
			VHOST.to_string(),
			message.exchange.to_string(),
			message.routing_key.to_string(),
		);

		payloads
			.send(payload)
			.await
			.expect("Could not send payload for processing");

		message
			.acker
			.ack(BasicAckOptions { multiple: true })
			.await
			.expect("Failed to ack");
	}
}
