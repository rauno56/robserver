mod types;
mod definitions;

use std::collections::HashSet;

use futures_lite::StreamExt;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Queue};
use tokio::sync::mpsc;
use tokio::time;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info};

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

#[derive(PartialEq, Eq, Hash, Debug)]
struct BindableEx {
	pub name: String,
	pub routing_key: String,
}

struct Binder {
	connection: Connection,
	channel: Channel,
	queue: String,
	bound: HashSet<BindableEx>,
}

impl Binder {
	pub async fn new(connection: Connection, queue: String) -> Self {
		let channel = connection.create_channel().await.expect("create_channel");

		Binder { connection, channel, queue, bound: HashSet::new() }
	}

	pub async fn bind(&mut self, bindable: BindableEx) {
		if self.bound.contains(&bindable) {
			debug!(?bindable, "Already bound");
			return
		}

		match self.channel
			.queue_bind(
				&self.queue,
				&bindable.name,
				&bindable.routing_key,
				QueueBindOptions::default(),
				FieldTable::default(),
			)
			.await
		{
			Ok(_) => {
				info!(?bindable, "Successfully bound");
				self.bound.insert(bindable);
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
			binder.bind(BindableEx { name: ex, routing_key: "#".to_string() } ).await;
		}
	}

	// TODO: match on vhost
	const VHOST: &str = "/";
	const Q: &str = "queue";
	let definitions_url = config::get_definitions_url();
	let queue_name = config::get_queue();
	let mut interval = time::interval(Duration::from_millis(5000));

	loop {
			interval.tick().await;
			match get_definitions(&definitions_url).await {
				Ok(mut result) => {
					result.bindings.retain(|binding| binding.vhost == VHOST && !(binding.destination_type == Q && binding.destination == queue_name));

					for ex_index in 0..result.exchanges.len() {
						let ex = &result.exchanges[ex_index];
						if ex.vhost != VHOST {
							continue;
						}
						match ex.r#type {
							ExchangeType::Direct => {
								let mut found_count = 0;
								for binding_index in 0..result.bindings.len() {
									let binding = &result.bindings[binding_index];
									if &binding.source == &ex.name {
										binder.bind(BindableEx { name: ex.name.clone(), routing_key: binding.routing_key.clone() }).await;
										found_count += 1;
									}
								}
								if found_count == 0 {
									// Change to debug
									debug!(exchange=ex.name, "No bindings found", );
								} else {
									// Change to debug
									debug!(found_count, name=ex.name, "Bindings found");
								}
							}
							_ => {
								binder.bind(BindableEx { name: ex.name.clone(), routing_key: "#".to_string() }).await;
							}
						};
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
