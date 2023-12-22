use serde_json::Value;
use sqlx::postgres::PgQueryResult;
use sqlx::{types::BigDecimal, PgPool};
use std::error::Error;
use tokio::sync::mpsc;
use tracing::info;

use crate::config;
use crate::payload::Payload;
use std::collections::HashMap;

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

async fn insert_counts(
	conn: &PgPool,
	mut counts: HashMap<Payload, usize>,
) -> Result<PgQueryResult, sqlx::Error> {
	let mut id = Vec::with_capacity(counts.len());
	let mut vhost = Vec::with_capacity(counts.len());
	let mut exchange = Vec::with_capacity(counts.len());
	let mut json = Vec::with_capacity(counts.len());
	let mut count = Vec::with_capacity(counts.len());
	for (p, to_add) in counts.drain() {
		if to_add == 0 {
			continue;
		}
		id.push(BigDecimal::from(p.id));
		vhost.push(p.vhost);
		exchange.push(p.exchange);
		json.push(p.json);
		count.push(to_add as i32);
	}
	info!(len = id.len(), "Inserting/updating counts");
	sqlx::query!(
		r#"
		insert into data.entity as e (
			id, vhost, exchange, payload, count
		)
		select
			id, vhost, exchange, payload, count
		from (
			select
				unnest($1::numeric[]) as id,
				unnest($2::text[]) as vhost,
				unnest($3::text[]) as exchange,
				unnest($4::jsonb[]) as payload,
				unnest($5::integer[]) as count
		) as new
		on conflict
			on constraint entity_pkey
				do update set count = e.count + EXCLUDED.count, last_seen_at = now()
	"#,
		&id[..],
		&vhost[..],
		&exchange[..],
		&json[..],
		&count[..],
	)
	.execute(conn)
	.await
}

pub async fn consumer(mut rx: mpsc::Receiver<Payload>) {
	info!("Connecting...");
	let pool = PgPool::connect(config::psql::get_url().as_str())
		.await
		.expect("Failed to connect to Postgres");
	info!("Connected");

	let buffer_size = config::psql::get_max_query_size();
	let mut to_handle: Vec<Payload> = Vec::with_capacity(buffer_size);

	while let x = rx.recv_many(&mut to_handle, buffer_size).await {
		let mut counts_to_handle: HashMap<Payload, usize> = HashMap::with_capacity(x);
		info!(len = x, "Processing items");

		for payload in to_handle.drain(0..) {
			if let Some(c) = counts_to_handle.get_mut(&payload) {
				*c += 1;
			} else {
				counts_to_handle.insert(payload, 1);
			}
		}
		insert_counts(&pool, counts_to_handle).await;
		// slow queries down
		// tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
	}
}
