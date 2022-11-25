use std::collections::HashMap;

use anyhow::anyhow;
use reqwest::get;
use serde::{Deserialize, Serialize};

use crate::data::{Source, SpellCollection, SpellSchool};

//const API_ENDPOINT: &str = "https://5e.tools/data"; // Protected by Cloudflare, ugh
const API_ENDPOINT: &str = "https://5etools-mirror-1.github.io/data";

async fn get_index() -> anyhow::Result<HashMap<String, String>> {
	let res = get(format!("{}/spells/index.json", API_ENDPOINT)).await?;

	let map = serde_json::from_slice(&res.bytes().await?).map_err(|err| anyhow!(err))?;
	Ok(map)
}

// fn get_sources(ids: &[String]) -> Vec<Book> {
// 	ids.iter().filter_map(|str| get_source(str).ok()).collect()
// }

pub async fn get_source(id: &str) -> anyhow::Result<Book> {
	let index = get_index().await?;
	let file = index.get(id).unwrap();

	let resp = get(format!("{}/spells/{}", API_ENDPOINT, file)).await?;
	let mut obj: Book = serde_json::from_slice(&resp.bytes().await?)?;
	obj.id = String::from(id);

	Ok(obj)
}

#[derive(Debug, Serialize, Deserialize)]
struct Subclass {
	class: Class,
	subclass: Class,
}

#[derive(Debug, Serialize, Deserialize)]
struct Class {
	name: String,
	source: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Classes {
	from_class_list: Vec<Class>,
	from_class_list_variant: Vec<Class>,
	from_subclass: Vec<Subclass>,
}

impl From<Classes> for Vec<String> {
	fn from(val: Classes) -> Self {
		val.from_class_list.into_iter().map(|el| el.name).collect()
	}
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Meta {
	ritual: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Spell {
	name: String,
	level: u8,
	school: SpellSchool,
	classes: Classes,
	meta: Meta,
}

impl From<Spell> for crate::data::Spell {
	fn from(value: Spell) -> Self {
		Self {
			name: value.name,
			level: value.level,
			school: value.school,
			classes: value.classes.into(),
			description: String::new(),
			ritual: value.meta.ritual,
		}
	}
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Book {
	id: String,
	#[serde(rename = "spell")]
	spells: Vec<Spell>,
}

impl From<Book> for SpellCollection {
	fn from(value: Book) -> Self {
		Self {
			id: Source::FiveE(value.id.clone()),
			name: value.id,
			image: None,
			spells: value.spells.into_iter().map(Into::into).collect(),
			spell_lists: HashMap::new(),
		}
	}
}
