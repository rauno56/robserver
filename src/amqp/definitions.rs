use super::types::{Definitions,Binding,Exchange};
use tracing::{debug, error};

pub async fn get_definitions(url: &str) -> Result<Definitions, Box<dyn std::error::Error>> {
	debug!("Requesting definitions for new bindings");
	let body = reqwest::get(format!("{}/exchanges", url)).await?.text().await?;

	let exchanges = match serde_json::from_str::<Vec<Exchange>>(&body) {
		Ok(res) => Ok(res),
		Err(error) => {
			error!(?body, "Failed to parse JSON");
			Err(Box::new(error))
		}
	}?;

	let body = reqwest::get(format!("{}/bindings", url)).await?.text().await?;
	let bindings = match serde_json::from_str::<Vec<Binding>>(&body) {
		Ok(res) => Ok(res),
		Err(error) => {
			error!(?body, "Failed to parse JSON");
			Err(Box::new(error))
		}
	}?;

	Ok(Definitions {
			exchanges,
			bindings,
		})
}
