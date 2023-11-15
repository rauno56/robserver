mod amqp;
mod config;
mod db;
mod hash;
mod payload;

use serde_json::Result;
use tokio::sync::mpsc;

use payload::Payload;

#[tokio::main]
async fn main() -> Result<()> {
	config::init();

	let (tx, rx) = mpsc::channel::<Payload>(config::get_buffer_size());

	let listener = amqp::listen_messages(tx);
	let consumer = db::consumer(rx);

	let _result = tokio::join!(listener, consumer);

	Ok(())
}
