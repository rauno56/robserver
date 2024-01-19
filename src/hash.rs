use std::hash::{Hash, Hasher};

use serde_json::Value;
use tracing::debug;

pub fn hash_object<T: Hasher>(obj: &Value, s: T) -> T {
	let mut state: T = s;
	if let Value::Object(x) = obj {
		'>'.hash(&mut state);
		for (key, value) in x.iter() {
			debug!("< {key}: {value}");
			key.hash(&mut state);
			state = hash_object(value, state);
		}
	}
	state
}

#[cfg(test)]
mod tests {
	use super::*;

	use std::collections::hash_map::DefaultHasher;
	use std::{assert_ne, hint::black_box};

	fn str_to_payload_hash(input: &str) -> u64 {
		debug!("hashing:\n{}", input);
		let s = DefaultHasher::new();
		let parsed: Value = serde_json::from_str(input).unwrap();
		hash_object(&parsed, s).finish()
	}

	const DATA: &str = r#"
		{
			"name": "John Doe",
			"age": 43,
			"phones": [
				"+44 1234567",
				"+44 2345678"
			],
			"deep": { "deep": { "object": { "b": "b_value", "a": 3123 } } },
			"a": 123
		}"#;
	const DATA_REORDERED: &str = r#"
		{
			"deep": { "deep": { "object": { "b": "b_value", "a": 123 } } },
			"a": 123,
			"age": 213,
			"phones": [
				"+44",
				"+44d"
			],
			"name": "Jane"
		}"#;
	const DATA_CHANGED: &str = r#"
		{
			"deep": { "deep": { "object": { "b": "b_value", "a": 123 } } },
			"firstName": "Jane",
			"a": 123,
			"age": 213,
			"phones": [
				"+44",
				"+44d"
			]
		}"#;
	const DATA_DEEPER_PROP: &str = r#"
		{
			"deep": { "deep": { "object": { "b": "b_value", "a": 123, "c": 123 } } },
			"name": "Jane",
			"a": 123,
			"age": 213,
			"phones": [
				"+44",
				"+44d"
			]
		}"#;
	const DATA_A: &str = r#"
		{
			"a": 123,
			"b": 123,
			"c": 123
		}"#;
	const DATA_B: &str = r#"
		{
			"a": 123,
			"b": { "c": 123 }
		}"#;

	#[test]
	fn output_data() {
		assert_eq!(str_to_payload_hash(DATA), 17100678054633095266);
		assert_eq!(str_to_payload_hash(DATA_REORDERED), 17100678054633095266);
	}

	#[test]
	fn output_data_changed() {
		assert_eq!(str_to_payload_hash(DATA_CHANGED), 5442145866395915742);
		assert_eq!(str_to_payload_hash(DATA_DEEPER_PROP), 18213723966592070427);
	}

	#[test]
	fn output_data_nested() {
		assert_eq!(str_to_payload_hash(DATA_A), 3133080193286829322);
		assert_eq!(str_to_payload_hash(DATA_B), 4013296165860414213);
	}

	#[test]
	fn recognize_nested() {
		assert_ne!(str_to_payload_hash(DATA_A), str_to_payload_hash(DATA_B));
	}

	#[test]
	fn reordered_eq() {
		assert_eq!(
			str_to_payload_hash(DATA),
			str_to_payload_hash(DATA_REORDERED)
		);
	}

	#[ignore = "ignore benchmarks for faster test runs"]
	#[test]
	fn bench() {
		for _ in 0..10_000 {
			black_box(str_to_payload_hash(DATA));
			black_box(str_to_payload_hash(DATA_REORDERED));
			black_box(str_to_payload_hash(DATA_CHANGED));
			black_box(str_to_payload_hash(DATA_DEEPER_PROP));
		}
	}
}
