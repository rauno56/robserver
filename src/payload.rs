use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use crate::hash::hash_object;

#[derive(Debug)]
pub struct Payload {
	data: Vec<u8>,
	pub vhost: String,
	pub exchange: String,
}

impl Payload {
	pub fn new(data: Vec<u8>, vhost: String, exchange: String) -> Payload {
		Payload {
			data,
			vhost,
			exchange,
		}
	}

	pub fn json(&self) -> Value {
		serde_json::from_slice(&self.data).unwrap()
	}

	pub fn hash(&self) -> (u64, Value) {
		let json = self.json();
		let s = DefaultHasher::new();
		let hash = hash_object(&json, s).finish();
		(hash, json)
	}
}
