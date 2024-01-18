use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Binding {
	pub source: String,
	pub vhost: String,
	pub destination: String,
	pub destination_type: String,
	pub routing_key: String,
	// arguments: ??,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Exchange {
	pub name: String,
	pub vhost: String,
	pub r#type: String,
	pub auto_delete: bool,
	pub internal: bool,
	// arguments: ??,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Definitions {
	pub bindings: Vec<Binding>,
	pub exchanges: Vec<Exchange>,
}
