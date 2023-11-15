use serde_json::Value;
use sqlx::postgres::PgQueryResult;
use sqlx::{types::BigDecimal, PgPool};
use std::error::Error;
use tokio::sync::mpsc;
use tracing::info;

use crate::config;
use crate::payload::Payload;
use std::collections::HashMap;

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
	record(
		conn,
		payload.id,
		payload.vhost.as_str(),
		payload.exchange.as_str(),
		&payload.json,
	)
	.await?;
	Ok(())
}

async fn update_counts(
	conn: &PgPool,
	mut counts: HashMap<Payload, usize>,
) -> Result<PgQueryResult, sqlx::Error> {
	let mut id = Vec::with_capacity(counts.len());
	let mut vhost = Vec::with_capacity(counts.len());
	let mut exchange = Vec::with_capacity(counts.len());
	let mut count = Vec::with_capacity(counts.len());
	for (p, to_add) in counts.drain() {
		if to_add == 0 {
			continue;
		}
		id.push(BigDecimal::from(p.id));
		vhost.push(p.vhost);
		exchange.push(p.exchange);
		count.push(to_add as i32);
	}
	info!("Updating counts for {:?} items", id.len());
	sqlx::query!(
		r#"
		update data.entity as e
		set
			count = count + new.add,
			last_seen_at = now()
		from (
			select
				unnest($1::numeric[]) as id,
				unnest($2::text[]) as vhost,
				unnest($3::text[]) as exchange,
				unnest($4::integer[]) as add
		) as new
		where (e.id, e.vhost, e.exchange) = (new.id, new.vhost, new.exchange)
	"#,
		&id[..],
		&vhost[..],
		&exchange[..],
		&count[..],
	)
	.execute(conn)
	.await
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
		let mut counts_to_handle: HashMap<Payload, usize> = HashMap::with_capacity(x);
		info!("Processing {x} items");
		let mut new_items_count = 0;

		// TODO: insert many with a single request using recv_many
		for payload in to_handle.drain(0..) {
			// let map_payload: Payload = payload.to_owned();
			if let Some(c) = counts_to_handle.get_mut(&payload) {
				*c += 1;
			} else {
				let _result = record_payload(&pool, &payload).await;
				new_items_count += 1;
				counts_to_handle.insert(payload, 0);
			}
		}
		info!("Payload recorded for {:?} items", new_items_count);
		update_counts(&pool, counts_to_handle).await;
		// info!(count=counts_to_handle.to_string(), "requests done");
		// to_handle.clear();
		// tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
	}
}
