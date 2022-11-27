pub mod sources;

use std::{collections::HashMap, hash::Hash};

use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, Serialize, Deserialize, poise::ChoiceParameter, PartialEq, Eq)]
#[serde(from = "String")]
pub enum SpellSchool {
	#[default]
	Abjuration,
	Conjuration,
	Divination,
	Enchantment,
	Evocation,
	Illusion,
	Necromancy,
	Transmutation,
}

impl SpellSchool {
	pub fn all() -> [SpellSchool; 8] {
		[
			Self::Abjuration,
			Self::Conjuration,
			Self::Divination,
			Self::Enchantment,
			Self::Evocation,
			Self::Illusion,
			Self::Necromancy,
			Self::Transmutation,
		]
	}
	fn id(&self) -> char {
		match *self {
			SpellSchool::Abjuration => 'A',
			SpellSchool::Conjuration => 'C',
			SpellSchool::Divination => 'D',
			SpellSchool::Enchantment => 'E',
			SpellSchool::Evocation => 'V',
			SpellSchool::Illusion => 'I',
			SpellSchool::Necromancy => 'N',
			SpellSchool::Transmutation => 'T',
		}
	}
}

impl From<SpellSchool> for char {
	fn from(val: SpellSchool) -> Self {
		val.id()
	}
}

impl From<String> for SpellSchool {
	fn from(value: String) -> Self {
		match value.as_str() {
			"A" => Self::Abjuration,
			"C" => Self::Conjuration,
			"D" => Self::Divination,
			"E" => Self::Enchantment,
			"V" => Self::Evocation,
			"I" => Self::Illusion,
			"N" => Self::Necromancy,
			"T" => Self::Transmutation,
			_ => {
				for school in SpellSchool::all() {
					if school.name().to_lowercase().contains(&value.to_lowercase()) {
						return school
					}
				}
				Self::default()
			},
		}
	}
}

// impl From<String> for SpellSchool {
// 	fn from(value: String) -> Self {
// 		match value {

// 		}
// 	}
// }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Spell {
	pub name: String,
	pub level: u8,
	pub school: SpellSchool,
	pub classes: Vec<String>,

	pub description: String,
	pub ritual: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpellCollection {
	id: Source,
	pub name: String,
	image: Option<String>,

	pub spells: Vec<Spell>,
	pub spell_lists: HashMap<String, Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Source {
	Avrae(String),
	FiveE(String),
	Json(String),
}
