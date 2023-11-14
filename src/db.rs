use serde_json::Value;
use sqlx::postgres::PgQueryResult;
use sqlx::{types::BigDecimal, PgPool};
use std::error::Error;
use tokio::sync::mpsc;
use tracing::{span, info, Level};

use crate::payload::Payload;
use crate::config;

async fn record(
	conn: &PgPool,
	id: u64,
	vhost: &str,
	exchange: &str,
	json: &Value,
) -> Result<PgQueryResult, sqlx::Error> {
	sqlx::query!(
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
	.await
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
	let pool = PgPool::connect(config::psql::get_url().as_str())
		.await
		.expect("connect");

	let buffer_size = config::get_buffer_size();
	let mut to_handle: Vec<Payload> = Vec::new();

	// while let Some(payload) = rx.recv().await {
	while let x = rx.recv_many(&mut to_handle, buffer_size).await {
		info!("Got {:?} to buffer({})", x, to_handle.len());
		// TODO: insert many with a single request using recv_many
		for payload in to_handle.iter() {
			let result = record_payload(&pool, payload).await;
		}
		info!("requests done");
		to_handle.clear();
		// tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
	}
}
