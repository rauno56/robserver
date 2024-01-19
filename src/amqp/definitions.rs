use super::types::Definitions;
use tracing::{debug, error};

pub async fn get_definitions(url: &str) -> Result<Definitions, Box<dyn std::error::Error>> {
	debug!("Requesting definitions for new bindings");
	let body = reqwest::get(url).await?.text().await?;

	match serde_json::from_str(&body) {
		Ok(res) => Ok(res),
		Err(error) => {
			error!(?body, "Failed to parse JSON");
			Err(Box::new(error))
		}
	}
}
