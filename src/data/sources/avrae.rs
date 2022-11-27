use std::collections::HashMap;

use anyhow::anyhow;
use reqwest::get;
use serde::Deserialize;

use crate::data::{Source, Spell, SpellCollection, SpellSchool};

const API_ENDPOINT: &str = "https://api.avrae.io";

// lazy_static! {
// 	static ref SRD: AvraeTome = futures::executor::block_on(get_srd()).unwrap();
// }

pub async fn get_tome(id: &str) -> anyhow::Result<AvraeTome> {
	log::info!("Grabbing: {id}");
	if id.eq("srd") {
		return get_srd().await;
	}

	let resp = get(format!("{API_ENDPOINT}/homebrew/spells/{id}")).await?;

	let api_response: AvraeApiResponse<AvraeTome> = serde_json::from_slice(&resp.bytes().await?)?;

	let Some(data) = api_response.data else {
        return Err(anyhow!("{}", api_response.error.unwrap_or_else(|| "Expected error message from avrae api.".to_string())));  
    };
	log::debug!("Success: {data:?}");
	Ok(data)
}

pub async fn get_srd() -> anyhow::Result<AvraeTome> {
	let resp = get(format!("{API_ENDPOINT}/homebrew/spells/srd")).await?;

	let api_response: AvraeApiResponse<Vec<AvraeSpell>> =
		serde_json::from_slice(&resp.bytes().await?)?;

	let Some(spells) = api_response.data else {
        return Err(anyhow!("{}", api_response.error.unwrap_or_else(|| "Expected error message from avrae api.".to_string())));  
    };

	Ok(AvraeTome {
		id: "srd".to_string(),
		name: "SRD".to_string(),
		image: String::new(),
		spells,
	})
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AvraeApiResponse<T> {
	error: Option<String>,
	data: Option<T>,
}

// #[derive(Debug, Clone, Deserialize)]
// struct SpellComponents {
// 	verbal: bool,
// 	somatic: bool,
// 	material: String,
// }

#[derive(Debug, Clone, Deserialize)]
struct AvraeSpell {
	name: String,
	level: u8,
	school: SpellSchool,
	classes: String,
	// subclasses: String,
	#[serde(rename = "casttime")]
	// cast_time: String,
	// range: String,
	// components: SpellComponents,
	// duration: String,
	ritual: bool,
	description: String,
}

impl From<AvraeSpell> for Spell {
	fn from(value: AvraeSpell) -> Self {
		Self {
			name: value.name,
			level: value.level,
			school: value.school,
			classes: value.classes.split(", ").map(Into::into).collect(),
			description: value.description,
			ritual: value.ritual,
		}
	}
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[allow(clippy::module_name_repetitions)]
pub struct AvraeTome {
	#[serde(rename = "_id")]
	id: String,

	name: String,
	image: String,

	spells: Vec<AvraeSpell>,
}

impl From<AvraeTome> for SpellCollection {
	fn from(value: AvraeTome) -> Self {
		Self {
			id: Source::Avrae(value.id),
			name: value.name,
			image: if value.image.eq("") {
				None
			} else {
				Some(value.image)
			},
			spells: value.spells.into_iter().map(Into::into).collect(),
			spell_lists: HashMap::new(),
		}
	}
}
