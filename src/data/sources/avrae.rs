use std::collections::HashMap;

use anyhow::anyhow;
use reqwest::get;
use serde::Deserialize;

use crate::data::{Source, Spell, SpellCollection, SpellSchool};

const API_ENDPOINT: &str = "https://api.avrae.io";

pub async fn get_tome(id: &str) -> anyhow::Result<AvraeTome> {
	let resp = get(format!("{}/homebrew/spells/{}", API_ENDPOINT, id)).await?;

	let api_response: AvraeApiResponse<AvraeTome> = serde_json::from_slice(&resp.bytes().await?)?;

	let Some(data) = api_response.data else {
        return Err(anyhow!("{}", api_response.error.unwrap_or_else(|| "Expected error message from avrae api.".to_string())));  
    };

	Ok(data)
}

pub async fn get_srd() -> anyhow::Result<AvraeTome> {
	let resp = get(format!("{}/homebrew/spells/srd", API_ENDPOINT)).await?;

	let api_response: AvraeApiResponse<Vec<AvraeSpell>> =
		serde_json::from_slice(&resp.bytes().await?)?;

	let Some(spells) = api_response.data else {
        return Err(anyhow!("{}", api_response.error.unwrap_or_else(|| "Expected error message from avrae api.".to_string())));  
    };

	Ok(AvraeTome {
		id: "srd".to_string(),
		name: "SRD".to_string(),
		image: "".to_string(),
		spells,
	})
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AvraeApiResponse<T> {
	error: Option<String>,
	data: Option<T>,
}

#[derive(Debug, Deserialize)]
struct SpellComponents {
	verbal: bool,
	somatic: bool,
	material: String,
}

#[derive(Debug, Deserialize)]
struct AvraeSpell {
	name: String,
	level: u8,
	school: SpellSchool,
	classes: String,
	subclasses: String,
	#[serde(rename = "casttime")]
	cast_time: String,
	range: String,
	components: SpellComponents,
	duration: String,
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

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
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
