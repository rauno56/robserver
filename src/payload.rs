use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::hash::hash_object;

#[derive(Debug, Clone, PartialEq)]
pub enum Data {
	Json(Value),
	Raw(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct Payload {
	pub content: Data,
	pub id: u64,
	pub vhost: String,
	pub exchange: String,
}

impl Payload {
	pub fn new(data: Vec<u8>, vhost: String, exchange: String) -> Payload {
		let Ok(json) = serde_json::from_slice(&data) else {
			return Payload {
				content: Data::Raw(data),
				id: 0,
				vhost,
				exchange,
			};
		};
		let s = DefaultHasher::new();
		let id = hash_object(&json, s).finish();
		Payload {
			content: Data::Json(json),
			id,
			vhost,
			exchange,
		}
	}
}

impl Hash for Payload {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id.hash(state);
		self.vhost.hash(state);
		self.exchange.hash(state);
	}
}

impl PartialEq for Payload {
	fn eq(&self, other: &Payload) -> bool {
		self.id == other.id && self.vhost == other.vhost && self.exchange == other.exchange
	}
}

impl Eq for Payload {}

#[cfg(test)]
mod tests {
	use std::collections::{HashMap, HashSet};

	use super::*;

	// `{"foo":"bar","prop0":10}`
	const V1: &[u8] = &[
		123, 34, 102, 111, 111, 34, 58, 34, 98, 97, 114, 34, 44, 34, 112, 114, 111, 112, 48, 34,
		58, 49, 48, 125,
	];
	// `{"foo":"bar","prop0":13}`
	const V2: &[u8] = &[
		123, 34, 102, 111, 111, 34, 58, 34, 98, 97, 114, 34, 44, 34, 112, 114, 111, 112, 48, 34,
		58, 49, 51, 125,
	];
	// `{"foo":"bar","prop1":13}`
	const V3: &[u8] = &[
		123, 34, 102, 111, 111, 34, 58, 34, 98, 97, 114, 34, 44, 34, 112, 114, 111, 112, 49, 34,
		58, 49, 51, 125,
	];
	// `foo":"bar","prop1":13}`
	const INVALID: &[u8] = &[
		102, 111, 111, 34, 58, 34, 98, 97, 114, 34, 44, 34, 112, 114, 111, 112, 49, 34, 58, 49, 51,
		125,
	];

	const VHOST1: &str = "/1";
	const VHOST2: &str = "/2";

	const EX1: &str = "A";
	const EX2: &str = "B";

	fn hash(p: &Payload) -> u64 {
		let mut s = DefaultHasher::new();
		p.hash(&mut s);
		s.finish()
	}

	#[test]
	fn payload_cmp() {
		assert_eq!(
			Payload::new(V1.to_vec(), String::from(VHOST1), String::from(EX1)),
			Payload::new(V1.to_vec(), String::from(VHOST1), String::from(EX1))
		);
		assert_ne!(
			Payload::new(V1.to_vec(), String::from(VHOST1), String::from(EX1)),
			Payload::new(V1.to_vec(), String::from(VHOST2), String::from(EX1))
		);
		assert_ne!(
			Payload::new(V1.to_vec(), String::from(VHOST1), String::from(EX1)),
			Payload::new(V1.to_vec(), String::from(VHOST1), String::from(EX2))
		);
	}

	#[test]
	fn payload_cmp_json_collapse() {
		assert_eq!(
			Payload::new(V1.to_vec(), String::from(VHOST1), String::from(EX1)),
			Payload::new(V2.to_vec(), String::from(VHOST1), String::from(EX1))
		);
		assert_ne!(
			Payload::new(V1.to_vec(), String::from(VHOST1), String::from(EX1)),
			Payload::new(V3.to_vec(), String::from(VHOST1), String::from(EX1))
		);
	}

	#[test]
	fn payload_invalid() {
		let payload = Payload::new(INVALID.to_vec(), String::from(VHOST1), String::from(EX1));

		assert_eq!(
			payload.content,
			Data::Raw(r#"foo":"bar","prop1":13}"#.into())
		);
		assert_eq!(payload.id, 0);

		assert_eq!(payload.vhost, String::from(VHOST1));
		assert_eq!(payload.exchange, String::from(EX1));
	}

	#[test]
	fn hashing() {
		let p1 = Payload::new(V1.to_vec(), String::from(VHOST2), String::from(EX2));
		let p2 = Payload::new(V1.to_vec(), String::from(VHOST1), String::from(EX2));
		let p3 = Payload::new(V1.to_vec(), String::from(VHOST2), String::from(EX1));

		let p4 = Payload::new(V2.to_vec(), String::from(VHOST2), String::from(EX2));
		let p5 = Payload::new(V3.to_vec(), String::from(VHOST2), String::from(EX2));

		assert_eq!(hash(&p1), hash(&p1));
		assert_ne!(hash(&p1), hash(&p2));
		assert_ne!(hash(&p1), hash(&p3));

		assert_eq!(hash(&p1), hash(&p4));
		assert_ne!(hash(&p1), hash(&p5));
	}

	#[test]
	fn hash_set_usage() {
		let mut set = HashSet::with_capacity(20);

		set.insert(Payload::new(
			V1.to_vec(),
			String::from(VHOST1),
			String::from(EX1),
		));
		assert_eq!(set.len(), 1);

		set.insert(Payload::new(
			V1.to_vec(),
			String::from(VHOST1),
			String::from(EX1),
		));
		assert_eq!(set.len(), 1);

		set.insert(Payload::new(
			V2.to_vec(),
			String::from(VHOST1),
			String::from(EX1),
		));
		assert_eq!(set.len(), 1);

		set.insert(Payload::new(
			V3.to_vec(),
			String::from(VHOST1),
			String::from(EX1),
		));
		assert_eq!(set.len(), 2);

		set.insert(Payload::new(
			V1.to_vec(),
			String::from(VHOST1),
			String::from(EX1),
		));
		assert_eq!(set.len(), 2);

		set.insert(Payload::new(
			V1.to_vec(),
			String::from(VHOST2),
			String::from(EX1),
		));
		assert_eq!(set.len(), 3);

		set.insert(Payload::new(
			V1.to_vec(),
			String::from(VHOST1),
			String::from(EX2),
		));
		assert_eq!(set.len(), 4);

		set.insert(Payload::new(
			V1.to_vec(),
			String::from(VHOST2),
			String::from(EX2),
		));
		assert_eq!(set.len(), 5);
	}

	#[test]
	fn hash_map_usage() {
		let mut map = HashMap::new();

		map.insert(
			Payload::new(V1.to_vec(), String::from(VHOST1), String::from(EX1)),
			1,
		);
		assert_eq!(map.len(), 1);

		map.insert(
			Payload::new(V1.to_vec(), String::from(VHOST1), String::from(EX1)),
			2,
		);
		assert_eq!(map.len(), 1);

		map.insert(
			Payload::new(V2.to_vec(), String::from(VHOST1), String::from(EX1)),
			3,
		);
		assert_eq!(map.len(), 1);

		map.insert(
			Payload::new(V3.to_vec(), String::from(VHOST1), String::from(EX1)),
			4,
		);
		assert_eq!(map.len(), 2);

		map.insert(
			Payload::new(V1.to_vec(), String::from(VHOST1), String::from(EX1)),
			5,
		);
		assert_eq!(map.len(), 2);
	}
}
