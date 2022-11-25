use std::collections::HashMap;

use reqwest::Url;
use serde::Deserialize;

use crate::data::{Source, SpellCollection};

pub async fn get_json(url: Url) -> anyhow::Result<JsonTome> {
	let str = url.to_string();
	let resp = reqwest::get(url).await?;

	let mut tome: JsonTome = serde_json::from_slice(&resp.bytes().await?)?;
	tome.url = str;
	Ok(tome)
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct JsonTome {
	url: String,

	spell_lists: HashMap<String, Vec<String>>,
}

impl From<JsonTome> for SpellCollection {
	fn from(value: JsonTome) -> Self {
		let name = value
			.url
			.split('/')
			.last()
			.unwrap_or(&value.url)
			.to_string();
		Self {
			id: Source::Json(value.url),
			name,
			image: None,
			spells: Vec::new(),
			spell_lists: value.spell_lists,
		}
	}
}
