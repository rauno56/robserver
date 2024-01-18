use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de;

#[derive(Serialize, Deserialize, Debug)]
pub struct Binding {
	pub source: String,
	pub vhost: String,
	pub destination: String,
	pub destination_type: String,
	pub routing_key: String,
	// arguments: ??,
}

#[derive(Debug)]
pub enum ExchangeType {
	Direct,
	Headers,
	Topic,
	Fanout,
}

impl Serialize for ExchangeType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        serializer.serialize_str(match *self {
            ExchangeType::Direct => "direct",
            ExchangeType::Headers => "headers",
            ExchangeType::Topic => "topic",
            ExchangeType::Fanout => "fanout",
            // ExchangeType::Other(ref other) => other,
        })
    }
}

impl<'de> Deserialize<'de> for ExchangeType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "direct" => Ok(ExchangeType::Direct),
            "headers" => Ok(ExchangeType::Headers),
            "topic" => Ok(ExchangeType::Topic),
            "fanout" => Ok(ExchangeType::Fanout),
            _ => Err(de::Error::custom(format!("Invalid exchange type: {}", s))),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Exchange {
	pub name: String,
	pub vhost: String,
	pub r#type: ExchangeType,
	pub auto_delete: bool,
	pub internal: bool,
	// arguments: ??,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Definitions {
	pub bindings: Vec<Binding>,
	pub exchanges: Vec<Exchange>,
}
