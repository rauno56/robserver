use url::{ParseError, Url};

pub fn init() {
	if std::env::var("RUST_LOG").is_err() {
		std::env::set_var("RUST_LOG", "info");
	}
	tracing_subscriber::fmt::init();
}

pub fn get_buffer_size() -> usize {
	std::env::var("ROBSERVER_BUFFER_SIZE").map_or(10_000, |v| {
		v.parse::<usize>().expect("invalid ROBSERVER_BUFFER_SIZE")
	})
}

pub mod amqp {
	use super::definitions_url_from_amqp_url;

	pub fn get_url() -> String {
		std::env::var("ROBSERVER_AMQP_ADDR")
			.unwrap_or_else(|_| "amqp://guest:guest@127.0.0.1:5672/%2f".into())
	}

	pub fn get_api_url() -> String {
		std::env::var("ROBSERVER_AMQP_API_ADDR")
			.or_else(|_| definitions_url_from_amqp_url(get_url()))
			.unwrap_or_else(|_| "http://guest:guest@127.0.0.1:15672/api".into())
	}

	pub fn get_exchanges() -> Vec<String> {
		let exchanges = std::env::var("ROBSERVER_LISTEN_EX")
			.unwrap_or_else(|_| "amq.direct,amq.fanout,amq.headers,amq.topic".into());

		exchanges
			.split(',')
			.map(str::to_string)
			.filter(|x| !x.is_empty())
			.collect()
	}

	pub fn get_prefetch() -> u16 {
		std::env::var("ROBSERVER_PREFETCH").map_or(100, |v| {
			v.parse::<u16>().expect("invalid ROBSERVER_PREFETCH")
		})
	}

	pub fn get_queue_max_length() -> u32 {
		std::env::var("ROBSERVER_QUEUE_MAX_LENGTH").map_or(100_000, |v| {
			v.parse::<u32>()
				.expect("invalid ROBSERVER_QUEUE_MAX_LENGTH")
		})
	}

	pub fn get_queue() -> String {
		std::env::var("ROBSERVER_QUEUE").unwrap_or_else(|_| "robserver.messages".into())
	}
}

pub mod psql {
	pub fn get_url() -> String {
		std::env::var("ROBSERVER_PG_ADDR")
			.or_else(|_| std::env::var("DATABASE_URL"))
			.unwrap_or_else(|_| "postgres://postgres@127.0.0.1/robserver".into())
	}

	pub fn get_max_query_size() -> usize {
		std::env::var("ROBSERVER_MAX_QUERY_SIZE").map_or(1_000, |v| {
			v.parse::<usize>()
				.expect("invalid ROBSERVER_MAX_QUERY_SIZE")
		})
	}

	pub fn get_query_delay() -> u64 {
		std::env::var("ROBSERVER_QUERY_DELAY").map_or(1000, |v| {
			v.parse::<u64>().expect("invalid ROBSERVER_QUERY_DELAY")
		})
	}
}

pub fn definitions_url_from_amqp_url(amqp_url: String) -> Result<String, ParseError> {
	let parsed = Url::parse(&amqp_url)?;

	match (parsed.username(), parsed.password(), parsed.host_str()) {
		(user, Some(pass), Some(host)) => Ok(format!("http://{user}:{pass}@{host}:15672/api")),
		_ => {
			// TODO: return error and let caller handle it
			Ok("http://guest:guest@127.0.0.1:15672/api".to_string())
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn def_from_amqp() {
		assert_eq!(
			definitions_url_from_amqp_url(String::from("amqp://guest:guest@127.0.0.1:5672/%2f"))
				.unwrap(),
			String::from("http://guest:guest@127.0.0.1:15672/api")
		);
		assert_eq!(
			definitions_url_from_amqp_url(String::from(
				"amqp://user1:pass2@some.host.com:5672/%2f"
			))
			.unwrap(),
			String::from("http://user1:pass2@some.host.com:15672/api")
		);
	}

	#[test]
	fn def_from_amqp_fallback() {
		assert_eq!(
			definitions_url_from_amqp_url(String::from("amqp://some.host.com:5672/%2f")).unwrap(),
			String::from("http://guest:guest@127.0.0.1:15672/api")
		);
	}
}
