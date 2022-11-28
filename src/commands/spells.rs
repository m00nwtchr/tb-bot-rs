use std::{collections::HashMap, sync::Arc};

use diesel::prelude::*;
use futures::{stream::FuturesUnordered, StreamExt};
use itertools::Itertools;
use lazy_static::lazy_static;
use poise::serenity_prelude::{self as serenity, CreateEmbed, GuildId};
use tokio::sync::Mutex;

use crate::{
	data::{sources, Spell, SpellCollection, SpellSchool},
	models::GuildTome,
	Context, Error,
};

lazy_static! {
	pub static ref SPELL_MAP: Arc<Mutex<HashMap<GuildId, SpellMap>>> =
		Arc::new(Mutex::new(HashMap::new()));
}

/// Lists spells for specified class and level (prefix command)
///
#[poise::command(prefix_command, ephemeral, rename = "sl")]
pub async fn spell_list_prefix(
	ctx: Context<'_>,
	#[autocomplete = "super::autocomplete_class"]
	#[description = "Class"]
	class: String,
	#[description = "Spell level"] level: Option<String>,
	#[rest]
	#[description = "Additional arguments"]
	args: Option<String>,
) -> Result<(), Error> {
	if let Some(args) = args {
		let mut args = args.split(' ');
		// log::info!("Spell List (Prefix): {ctx:?}");
		let ritual = args.clone().any(|el| el.contains("--ritual"));

		let spell_schools: Vec<_> = args
			.clone()
			.filter_map(|arg| {
				SpellSchool::all().into_iter().find(|school| {
					let str = format!("--{}", school.name().to_lowercase());
					arg.contains(str.as_str())
				})
			})
			.collect();

		let not_classes: Vec<String> = args
			.clone()
			.filter_map(|arg| {
				arg.split('!')
					.last()
					.map(String::from)
					.map(|str| str.to_lowercase())
			})
			.collect();

		spell_list(
			ctx,
			class,
			if level.is_some() {
				level
			} else {
				args.next().map(String::from)
			},
			ritual,
			spell_schools,
			not_classes,
		)
		.await
	} else {
		spell_list(ctx, class, None, false, vec![], vec![]).await
	}
}

/// Lists spells for specified class and level (slash command)
#[poise::command(slash_command, ephemeral, rename = "spells")]
pub async fn spell_list_slash(
	ctx: Context<'_>,
	#[autocomplete = "super::autocomplete_class"]
	#[description = "Class"]
	class: String,
	#[description = "Spell level"] level: Option<String>,
	#[description = "Filter spell schools (--<spell school> in additional args)"]
	spell_schools: Vec<SpellSchool>,
	#[description = "Only display ritual spells"]
	#[flag]
	ritual: bool,
	#[autocomplete = "super::autocomplete_class"]
	#[description = "Exclude spells which belong to this class's spell list (--!<class> in additional args)"]
	not_classes: Vec<String>,
	// #[rest]
	// #[description = "Additional arguments"]
	// args: Option<String>,
) -> Result<(), Error> {
	spell_list(ctx, class, level, ritual, spell_schools, not_classes).await
}

async fn spell_list(
	ctx: Context<'_>,
	class: String,
	level: Option<String>,
	ritual: bool,
	spell_schools: Vec<SpellSchool>,
	not_classes: Vec<String>,
) -> Result<(), Error> {
	ctx.defer_or_broadcast().await?;
	let guild_id = ctx.guild_id().unwrap();

	let (min_level, max_level) = if let Some(level) = level {
		if level.contains('-') {
			let mut spl = level.split('-');

			let (one, two): (Option<u8>, Option<u8>) = (
				spl.next().and_then(|el| el.parse().ok()),
				spl.next().and_then(|el| el.parse().ok()),
			);

			(one, two)
		} else {
			let p: Option<u8> = level.parse().ok();

			(p, p)
		}
	} else {
		(None, None)
	};

	let spell_map = build_spell_map(guild_id, ctx.data().db.clone(), false).await;
	let iter = spell_map
		.get_spells(&class.to_lowercase())
		.unwrap()
		.into_iter()
		.filter(|el| !ritual || el.ritual)
		.filter(|el| {
			if let Some(min_level) = min_level {
				el.level >= min_level
			} else {
				true
			}
		})
		.filter(|el| {
			if let Some(max_level) = max_level {
				el.level <= max_level
			} else {
				true
			}
		})
		.filter(|el| {
			if let Some(max_level) = max_level {
				el.level <= max_level
			} else {
				true
			}
		})
		.filter(|spell| {
			not_classes.is_empty()
				|| !spell
					.classes
					.iter()
					.any(|class| not_classes.contains(class))
		})
		.filter(|spell| spell_schools.is_empty() || spell_schools.contains(&spell.school));

	let list: Vec<String> = if min_level.is_some() && min_level.eq(&max_level) {
		iter.map(|el| &el.name)
			.cloned()
			.sorted_unstable()
			.chunks(20)
			.into_iter()
			.map(|mut c| c.join("\n"))
			.collect()
	} else {
		iter.sorted_unstable_by_key(|el| el.level)
			.group_by(|el| el.level)
			.into_iter()
			.flat_map(|(level, group)| {
				std::iter::once(format!("**Level {level} spells**"))
					.chain(group.map(|el| &el.name).cloned().sorted_unstable())
			})
			.chunks(20)
			.into_iter()
			.map(|mut c| c.join("\n"))
			.collect()
	};

	super::send_paginated_message(ctx, &list, CreateEmbed::default()).await?;

	Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct SpellMap {
	pub spells: Vec<Spell>,
	pub map: HashMap<String, Vec<usize>>,
}

impl SpellMap {
	pub fn add_spell(&mut self, spell: Spell) {
		let i = self.spells.len();

		for class in &spell.classes {
			if let Some(cl) = self.map.get_mut(&class.to_lowercase()) {
				cl.push(i);
			} else {
				self.map.insert(class.to_lowercase(), vec![i]);
			}
		}
		self.spells.push(spell);
	}

	pub fn get_spells(&self, class: &String) -> Option<Vec<&Spell>> {
		let vec = self.map.get(class)?;
		let vec: Vec<&Spell> = vec.iter().filter_map(|el| self.spells.get(*el)).collect();

		Some(vec)
	}

	pub fn get_classes(&self) -> Vec<&String> {
		self.map.keys().collect()
	}
}

pub async fn build_spell_map(
	guild_id: GuildId,
	conn: Arc<Mutex<MysqlConnection>>,
	rebuild: bool,
) -> SpellMap {
	use crate::schema::GuildTomes::dsl::*;

	if !rebuild {
		if let Some(spell_map) = SPELL_MAP.lock().await.get(&guild_id) {
			return spell_map.clone();
		}
	}

	let mut conn = conn.lock().await;
	let serenity::GuildId(gid) = guild_id;

	let mut tomes = GuildTomes
		.filter(guild.eq(gid))
		.load::<GuildTome>(&mut *conn)
		.expect("Error loading guild tomes.");

	tomes.push(GuildTome {
		id: 0,
		guild: 0,
		source: "srd".to_string(),
	});

	tomes.extend(
		crate::data::sources::get_5e_index()
			.await
			.keys()
			.into_iter()
			.filter(|k| !k.starts_with("UA"))
			.map(|key| GuildTome {
				id: 0,
				guild: 0,
				source: key.clone(),
			}),
	);

	let mut sm = SpellMap::default();
	let mut spells_futures: FuturesUnordered<_> = tomes.iter().map(get_spells).collect();

	let mut tomes: Vec<SpellCollection> = Vec::new();
	while let Some(res) = spells_futures.next().await {
		match res {
			Ok(collection) => tomes.push(collection),
			Err(err) => log::error!("Error getting spell source: {err}"),
		}
	}

	let spell_lists: HashMap<String, Vec<String>> =
		tomes.iter().flat_map(|e| e.spell_lists.clone()).collect();

	tomes
		.into_iter()
		.flat_map(|e| e.spells)
		.sorted_by(|a, b| a.name.cmp(&b.name))
		.dedup_by(|a, b| a.name.eq(&b.name))
		.for_each(|mut spell| {
			spell
				.classes
				.extend(spell_lists.iter().filter_map(|(k, v)| {
					if v.contains(&spell.name) {
						Some(k.clone())
					} else {
						None
					}
				}));

			sm.add_spell(spell);
		});

	SPELL_MAP.lock().await.insert(guild_id, sm.clone());

	sm
}

async fn get_spells(tome: &GuildTome) -> anyhow::Result<SpellCollection> {
	sources::get_spells(&tome.source).await
}
