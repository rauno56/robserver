mod types;
mod definitions;
mod payload_parser;
mod exchange_subscriber;

use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Queue};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tracing::info;

use crate::config::amqp as config;
use crate::payload::Payload;

use payload_parser::payload_parser;
use exchange_subscriber::exchange_subscriber;

// Vhost is currently hard-coded to "/"
const VHOST: &str = "/";
const CONSUMER_TAG: &str = "robserver.ct";

async fn declare_work_queue(channel: &Channel, queue_name: &str) -> Result<Queue, lapin::Error> {
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
