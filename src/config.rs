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
	pub fn get_url() -> String {
		std::env::var("ROBSERVER_AMQP_ADDR")
			.unwrap_or_else(|_| "amqp://guest:guest@127.0.0.1:5672/%2f".into())
	}

	pub fn get_exchanges() -> Vec<String> {
		let exchanges = std::env::var("ROBSERVER_LISTEN_EX")
			.unwrap_or_else(|_| "amq.direct,amq.fanout,amq.headers,amq.topic".into());

		exchanges.split(',').map(str::to_string).collect()
	}

	pub fn get_prefetch() -> u16 {
		std::env::var("ROBSERVER_PREFETCH").map_or(100, |v| {
			v.parse::<u16>().expect("invalid ROBSERVER_PREFETCH")
		})
	}

	pub fn get_queue_max_length() -> u32 {
		std::env::var("ROBSERVER_QUEUE_MAX_LENGTH").map_or(100_000, |v| {
			v.parse::<u32>().expect("invalid ROBSERVER_QUEUE_MAX_LENGTH")
		})
	}
}

pub mod psql {
	pub fn get_url() -> String {
		std::env::var("ROBSERVER_PG_ADDR")
			.unwrap_or_else(|_| "postgres://postgres@127.0.0.1/robserver".into())
	}

	pub fn get_max_query_size() -> usize {
		std::env::var("ROBSERVER_MAX_QUERY_SIZE").map_or(1_000, |v| {
			v.parse::<usize>().expect("invalid ROBSERVER_MAX_QUERY_SIZE")
		})
	}
}
