use serde_json::Value;
use sqlx::{types::BigDecimal, PgPool};
use std::error::Error;
use tokio::sync::mpsc;
use tracing::info;

use crate::payload::Payload;

const DB_CONNECTION_STRING: &str = "postgres://postgres@127.0.0.1/robserver";

async fn record(
	conn: &PgPool,
	id: u64,
	vhost: &str,
	exchange: &str,
	json: &Value,
) -> Result<(), Box<dyn Error>> {
	let nr = sqlx::query!(
		r#"
		insert into data.entity as e (
			id, vhost, exchange, payload
		) values (
			$1, $2, $3, $4
		)
		on conflict
			on constraint entity_pkey
				do update set count = e.count + 1, last_seen_at = now()
	"#,
		BigDecimal::from(id),
		vhost,
		exchange,
		json,
	)
	.execute(conn)
	.await?;
	info!("Inserted: {:?}", nr);
	Ok(())
}

async fn record_payload(conn: &PgPool, payload: &Payload) -> Result<(), Box<dyn Error>> {
	let (hash, json) = payload.hash();
	record(
		conn,
		hash,
		payload.vhost.as_str(),
		payload.exchange.as_str(),
		&json,
	)
	.await?;
	Ok(())
}

pub async fn consumer(mut rx: mpsc::Receiver<Payload>) {
	info!("connecting to DB....");
	let pool = PgPool::connect(DB_CONNECTION_STRING)
		.await
		.expect("connect");

	let mut to_handle: Vec<Payload> = Vec::new();

	// while let Some(payload) = rx.recv().await {
	while let x = rx.recv_many(&mut to_handle, 20).await {
		info!("Got {:?} to buffer({})", x, to_handle.len());
		// TODO: insert many with a single request using recv_many
		for payload in to_handle.iter() {
			let result = record_payload(&pool, payload).await;
			info!("Request result: {:?}", result);
		}
		to_handle.clear();
		tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
	}
}
