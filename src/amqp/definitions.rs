use tracing::debug;
use super::types::Definitions;

pub async fn get_definitions(url: &str) -> Result<Definitions, Box<dyn std::error::Error>> {
	debug!("Requesting definitions for new bindings");
	let res = reqwest::get(url).await?;
	let res: Definitions = res.json().await?;

	Ok(res)
}
