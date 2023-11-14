mod amqp;
mod db;
mod hash;
mod payload;

use serde_json::Result;
use tokio::sync::mpsc;

use payload::Payload;

#[tokio::main]
async fn main() -> Result<()> {
	if std::env::var("RUST_LOG").is_err() {
		std::env::set_var("RUST_LOG", "info");
	}
	tracing_subscriber::fmt::init();

	let (tx, rx) = mpsc::channel::<Payload>(6);

	let listener = amqp::listen_messages(tx);
	let consumer = db::consumer(rx);

	let _result = tokio::join!(listener, consumer);

	Ok(())
}
