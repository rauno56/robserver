mod amqp;
mod config;
mod db;
mod hash;
mod payload;

use tokio::sync::mpsc;

use payload::Payload;

#[tokio::main]
async fn main() {
	config::init();

	let (payload_tx, payload_rx) = mpsc::channel::<Payload>(config::get_buffer_size());

	let listener = amqp::listen_messages(payload_tx);
	let consumer = db::consumer(payload_rx);

	tokio::select!(
		_ = listener => {}
		_ = consumer => {}
	);
}
