use std::collections::HashSet;

use lapin::{options::*, types::FieldTable, Channel, Connection};
use tokio::time;
use tokio::time::Duration;
use tracing::{debug, error, info};

use crate::config::amqp as config;

use super::VHOST;
use super::types::*;
use super::definitions::get_definitions;

const Q: &str = "queue";
const ROUTING_KEY_WILDCARD: &str = "#";

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
			binder.bind(BindableEx { name: ex, routing_key: ROUTING_KEY_WILDCARD.to_string() } ).await;
		}
	}

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
									if binding.source == ex.name {
										binder.bind(BindableEx { name: ex.name.clone(), routing_key: binding.routing_key.clone() }).await;
										found_count += 1;
									}
								}
								if found_count == 0 {
									debug!(exchange=ex.name, "No bindings found", );
								} else {
									debug!(found_count, name=ex.name, "Bindings found");
								}
							}
							_ => {
								binder.bind(BindableEx { name: ex.name.clone(), routing_key: ROUTING_KEY_WILDCARD.to_string() }).await;
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
