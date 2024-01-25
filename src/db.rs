use std::collections::HashMap;

use serde_json::Value;
use sqlx::postgres::{PgPoolOptions, PgQueryResult};
use sqlx::Executor;
use sqlx::{types::BigDecimal, PgPool};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::config;
use crate::payload::{Data, Payload};

async fn insert_counts(
	conn: &PgPool,
	mut counts: HashMap<Payload, usize>,
) -> Result<PgQueryResult, sqlx::Error> {
	let mut id = Vec::with_capacity(counts.len());
	let mut vhost = Vec::with_capacity(counts.len());
	let mut exchange = Vec::with_capacity(counts.len());
	let mut json = Vec::with_capacity(counts.len());
	let mut raw: Vec<Option<String>> = Vec::with_capacity(counts.len());
	let mut routing_key: Vec<String> = Vec::with_capacity(counts.len());
	let mut count = Vec::with_capacity(counts.len());
	for (p, to_add) in counts.drain() {
		if to_add == 0 {
			continue;
		}
		id.push(BigDecimal::from(p.id));
		vhost.push(p.vhost);
		exchange.push(p.exchange);
		routing_key.push(p.routing_key);
		match p.content {
			Data::Json(value) => {
				json.push(Some(value));
				raw.push(None);
			}
			Data::Raw(value) => {
				json.push(None);
				raw.push(String::from_utf8(value).ok());
			}
		}
		count.push(to_add as i32);
	}
	info!(len = id.len(), "Inserting/updating counts");
	sqlx::query!(
		r#"
		insert into data.entity as e (
			id,
			vhost,
			exchange,
			payload,
			raw_payload,
			routing_key,
			count
		)
		select
			id,
			vhost,
			exchange,
			payload,
			raw_payload,
			routing_key,
			count
		from (
			select
				unnest($1::numeric[]) as id,
				unnest($2::text[]) as vhost,
				unnest($3::text[]) as exchange,
				unnest($4::jsonb[]) as payload,
				unnest($5::text[]) as raw_payload,
				unnest($6::text[]) as routing_key,
				unnest($7::integer[]) as count
		) as new
		on conflict
			on constraint entity_pkey
				do update set count = e.count + EXCLUDED.count, last_seen_at = now()
	"#,
		&id[..],
		&vhost[..],
		&exchange[..],
		&json[..] as &[Option<Value>],
		&raw[..] as &[Option<String>],
		&routing_key[..],
		&count[..],
	)
	.execute(conn)
	.await
}

pub async fn consumer(mut rx: mpsc::Receiver<Payload>) {
	info!("Connecting...");
	let pool = PgPoolOptions::new()
		.after_connect(|conn, _meta| {
			Box::pin(async move {
				conn.execute("SET application_name = 'robserver';").await?;

				Ok(())
			})
		})
		.connect(config::psql::get_url().as_str())
		.await
		.expect("Failed to connect to Postgres");
	info!("Connected");

	let buffer_size = config::psql::get_max_query_size();
	let mut to_handle: Vec<Payload> = Vec::with_capacity(buffer_size);

	loop {
		let x = rx.recv_many(&mut to_handle, buffer_size).await;
		if x == 0 {
			error!("Channel closed");
			break;
		}
		let mut counts_to_handle: HashMap<Payload, usize> = HashMap::with_capacity(x);
		info!(len = x, "Processing items");

		for payload in to_handle.drain(0..) {
			if let Some(c) = counts_to_handle.get_mut(&payload) {
				*c += 1;
			} else {
				counts_to_handle.insert(payload, 1);
			}
		}
		let _ = insert_counts(&pool, counts_to_handle)
			.await
			.expect("Failed to insert counts");
	}

	info!("DB worker finished");
}
