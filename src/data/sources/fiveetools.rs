use std::{collections::HashMap, error::Error, sync::Arc};

use anyhow::anyhow;
use lazy_static::lazy_static;
use reqwest::get;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, OnceCell};

use crate::data::{Source, SpellCollection, SpellSchool};

//const API_ENDPOINT: &str = "https://5e.tools/data"; // Protected by Cloudflare, ugh
const API_ENDPOINT: &str = "https://5etools-mirror-1.github.io/data";

type FiveEIndex = HashMap<String, String>;

lazy_static! {
	static ref INDEX: Arc<OnceCell<FiveEIndex>> = Arc::new(OnceCell::new());
	static ref SRC_CACHE: Arc<Mutex<HashMap<String, Book>>> = Arc::new(Mutex::new(HashMap::new()));	
}

async fn _get_index() -> anyhow::Result<FiveEIndex> {
	let res = get(format!("{API_ENDPOINT}/spells/index.json")).await?;

	let map = serde_json::from_slice(&res.bytes().await?).map_err(|err| anyhow!(err))?;
	Ok(map)
}

pub async fn get_index<'a>() -> &'a FiveEIndex {
	INDEX
		.get_or_init(async move || _get_index().await.unwrap())
		.await
}

pub async fn get_source(id: &str) -> anyhow::Result<Book> {
	let mut cache = SRC_CACHE.lock().await;
	if let Some(src) = cache.get(id) {
		return Ok(src.clone());
	}
	
	log::info!("Grabbing: {id}");
	let index = get_index().await;
	let file = index.get(id).unwrap();

	let resp = get(format!("{API_ENDPOINT}/spells/{file}")).await?;
	let mut obj: Book = serde_json::from_slice(&resp.bytes().await?)?;
	obj.id = String::from(id);

	cache.insert(String::from(id), obj.clone());
	Ok(obj)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Subclass {
	class: Class,
	subclass: Class,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Class {
	name: String,
	source: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Meta {
	ritual: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Spell {
	name: String,
	level: u8,
	// #[serde(deserialize_with = "deserialize_school")]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

// fn deserialize_school<'de, D>(de: D) -> Result<SpellSchool, D::Error>
// where
// 	D: Deserializer<'de>,
// {
// 	let c = de.deserialize_char(CharVisitor)?;
// 	Ok(c.into())
// }

// struct CharVisitor;

// impl<'de> serde::de::Visitor<'de> for CharVisitor {
// 	type Value = char;

// 	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
// 		formatter.write_str("a character")
// 	}

// 	#[inline]
// 	fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
// 	where
// 		E: serde::de::Error,
// 	{
// 		Ok(v)
// 	}

// 	#[inline]
// 	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
// 	where
// 		E: serde::de::Error,
// 	{
// 		let mut iter = v.chars();
// 		match (iter.next(), iter.next()) {
// 			(Some(c), None) => Ok(c),
// 			_ => Err(serde::de::Error::invalid_value(Unexpected::Str(v), &self)),
// 		}
// 	}
// }
