mod types;
mod definitions;

use std::collections::HashSet;

use futures_lite::StreamExt;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Queue};
use tokio::sync::mpsc;
use tokio::time;
use tokio::time::{timeout, Duration};
use tracing::{error, info};

use crate::config::amqp as config;
use crate::payload::Payload;

pub use types::*;
pub use definitions::get_definitions;

const CONSUMER_TAG: &str = "robserver.ct";

pub async fn declare_work_queue(channel: &Channel, queue_name: &str) -> Result<Queue, lapin::Error> {
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
		println!("processing...");
		let message = delivery.unwrap();

		let payload = Payload::new(message.data, "/".to_string(), message.exchange.to_string());

		payloads.send(payload)
			.await
			.expect("Could not send payload for processing");

		message
			.acker
			.ack(BasicAckOptions { multiple: true })
			.await
			.expect("Failed to ack");
	}
}

struct Binder {
	connection: Connection,
	channel: Channel,
	queue: String,
	_bound: HashSet<()>,
}

impl Binder {
	pub async fn new(connection: Connection, queue: String) -> Self {
		let channel = connection.create_channel().await.expect("create_channel");

		Binder { connection, channel, queue, _bound: HashSet::new() }
	}

	pub async fn bind(&mut self, ex: &str, routing_key: &str) {
		match self.channel
			.queue_bind(
				&self.queue,
				ex,
				routing_key,
				QueueBindOptions::default(),
				FieldTable::default(),
			)
			.await
		{
			Ok(_) => {
				info!(exchange = ex, "Successfully bound")
			}
			Err(_) => {
				self.channel = self.connection.create_channel().await.unwrap();
			}
		};
	}
}

pub async fn exchange_subscriber(conn: Connection) {
	let exchanges = config::get_exchanges();
	let work_queue = config::get_queue();
	let mut binder = Binder::new(conn, work_queue).await;

	if exchanges.is_empty() {
		info!("No exchanges to bind to");
	} else {
		for ex in exchanges {
			binder.bind(ex.as_str(), "#").await;
		}
	}

	let definitions_url = config::get_definitions_url();
	let queue_name = config::get_queue();
	let mut interval = time::interval(Duration::from_millis(5000));

	loop {
			interval.tick().await;
			match get_definitions(&definitions_url).await {
				Ok(result) => {
					for ex in result.exchanges {
						println!("bind to exchange: {:?}", ex);
					}

					for binding in result.bindings {
						if binding.destination != queue_name {
							println!("bind to exchange: {:?}", binding);
						}
					}
				},
				Err(error) => {
					error!(error, "Failed to get definitions");
				}
			}
	}
}

pub async fn listen_messages(payloads: mpsc::Sender<Payload>) {
	info!("Connecting...");
	let addr = config::get_url();
	let work_queue = config::get_queue();

	let conn = timeout(Duration::from_secs(5), async {
		Connection::connect(&addr, ConnectionProperties::default())
			.await
			.expect("Failed to connect to RabbitMQ")
	})
	.await
	.expect("Failed to connect to RabbitMQ");

	info!("Connected");

	let mut channel = conn.create_channel().await.expect("create_channel");
	let queue = declare_work_queue(&channel, &work_queue).await;
	match queue {
		Ok(_) => info!(?queue, "Declared queue"),
		Err(lapin::Error::ProtocolError(ref e)) if e.get_id() == 406 /* PRECONDITION FAILED */ => {
			info!("Queue already declared");
			channel = conn.create_channel().await.expect("create_channel");
		},
		Err(err) => panic!("Unrecoverable error declaring queue: {:?}", err),
	};

	let parser = payload_parser(payloads, channel);
	let subscriber = exchange_subscriber(conn);

	let _result = tokio::join!(parser, subscriber);
}
