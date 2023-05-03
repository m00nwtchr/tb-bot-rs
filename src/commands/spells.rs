use std::{collections::HashMap, sync::Arc};

use anyhow::anyhow;
use diesel::prelude::*;
use futures::{stream::FuturesUnordered, StreamExt};
use itertools::Itertools;
use lazy_static::lazy_static;
use poise::serenity_prelude::{self as serenity, CreateEmbed, GuildId, Typing};
use tokio::sync::{Mutex, RwLock};

use crate::{
	data::{sources, Spell, SpellCollection, SpellSchool},
	models::GuildTome,
	Context, Error,
};

/// Lists spells for specified class and level (prefix command)
///
#[poise::command(prefix_command, ephemeral, rename = "sl")]
pub async fn spell_list_prefix(
	ctx: Context<'_>,
	#[description = "Class"] class: String,
	#[description = "Spell level"] level: Option<String>,
	#[rest]
	#[description = "Additional arguments"]
	args: Option<String>,
) -> Result<(), Error> {
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

		// let level = if level.is_some() {
		// 	level
		// } else {
		// 	args.next().map(String::from)
		// };

		spell_list(
			ctx,
			class,
			min_level,
			max_level,
			ritual,
			spell_schools,
			not_classes,
		)
		.await
	} else {
		spell_list(ctx, class, min_level, max_level, false, vec![], vec![]).await
	}
}

/// Lists spells for specified class and level (slash command)
#[allow(clippy::too_many_arguments)]
#[poise::command(slash_command, ephemeral, rename = "spells")]
pub async fn spell_list_slash(
	ctx: Context<'_>,
	#[autocomplete = "super::autocomplete_class"]
	#[description = "Class"]
	class: String,
	#[autocomplete = "super::autocomplete_level"]
	#[description = "Spell level"]
	level: Option<u8>,
	#[autocomplete = "super::autocomplete_level"]
	#[description = "Minimum spell level"]
	mut min_level: Option<u8>,
	#[autocomplete = "super::autocomplete_level"]
	#[description = "Maximum spell level"]
	mut max_level: Option<u8>,
	#[description = "Filter spell schools"]
	#[autocomplete = "super::autocomplete_school"]
	// spell_schools: Vec<SpellSchool>,
	spell_school: Option<SpellSchool>,
	#[description = "Only display ritual spells"]
	#[flag]
	ritual: bool,
	#[autocomplete = "super::autocomplete_class"]
	#[description = "Exclude spells which belong to this class's spell list"]
	not_classes: Vec<String>,
	// #[rest]
	// #[description = "Additional arguments"]
	// args: Option<String>,
) -> Result<(), Error> {
	if min_level.is_some() && max_level.is_none() {
		max_level = Some(9);
	} else if max_level.is_some() && min_level.is_none() {
		min_level = Some(0);
	} else if level.is_some() {
		min_level = Some(level.unwrap());
		max_level = Some(level.unwrap());
	}

	spell_list(
		ctx,
		class,
		min_level,
		max_level,
		ritual,
		if let Some(spell_school) = spell_school {
			vec![spell_school]
		} else {
			Vec::new()
		},
		not_classes,
	)
	.await
}

async fn spell_list(
	ctx: Context<'_>,
	class: String,
	// level: Option<String>,
	min_level: Option<u8>,
	max_level: Option<u8>,
	ritual: bool,
	spell_schools: Vec<SpellSchool>,
	not_classes: Vec<String>,
) -> Result<(), Error> {
	ctx.defer_ephemeral().await?;
	let guild_id = ctx.guild_id().unwrap();

	let spell_map_map = ctx.data().spell_map.read().await;
	let spell_map = spell_map_map
		.get(&guild_id)
		.expect("Spell map not build for this guild yet.");

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

	super::send_paginated_message(ctx, list, CreateEmbed::default()).await?;

	Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct SpellMap {
	spells: Vec<Spell>,
	classes: Vec<String>,
	map: HashMap<String, Vec<usize>>,
}

impl SpellMap {
	pub fn add_spell(&mut self, spell: Spell) {
		let i = self.spells.len();

		if spell.classes.is_empty() {
			log::warn!("Spell with empty class list: {spell:?}");
		}
		for class in &spell.classes {
			if class.is_empty() {
				continue;
			}

			if !self.classes.contains(class) {
				self.classes.push(class.clone());
			}

			self.map
				.entry(class.to_lowercase())
				.or_insert(Vec::new())
				.push(i);
		}
		self.spells.push(spell);
	}

	pub fn get_spells(&self, class: &String) -> Option<Vec<&Spell>> {
		let vec = self.map.get(class)?;
		let vec: Vec<&Spell> = vec.iter().filter_map(|el| self.spells.get(*el)).collect();

		Some(vec)
	}

	pub fn get_classes(&self) -> &Vec<String> {
		&self.classes
	}
}

pub async fn build_spell_map(guild_id: GuildId, conn: Arc<Mutex<MysqlConnection>>) -> SpellMap {
	use crate::schema::GuildTomes::dsl::*;

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

			for class in &mut spell.classes {
				match class.as_str() {
					"Artificier" => *class = String::from("Artificer"),
					"Range" => *class = String::from("Ranger"),
					"Drud" => *class = String::from("Druid"),
					"Warloc" => *class = String::from("Warlock"),
					_ => {}
				};
			}

			sm.add_spell(spell);
		});

	sm
}

#[allow(clippy::too_many_arguments)]
#[poise::command(slash_command, prefix_command, ephemeral, check = "super::is_manager")]
pub async fn rebuild(ctx: Context<'_>) -> Result<(), Error> {
	if let Some(guild_id) = ctx.guild_id() {
		let _typing = ctx.defer_or_broadcast().await;
		let msg = ctx.say("Rebuilding spell lists...").await?;

		let sm = build_spell_map(guild_id, ctx.data().db.clone()).await;
		msg.edit(ctx, |m| {
			m.content(format!(
				"Done. {} classes and {} spells found.",
				sm.classes.len(),
				sm.spells.len()
			))
		})
		.await?;

		ctx.data().spell_map.write().await.insert(guild_id, sm);
	} else {
		ctx.say("Error: Must be ran in a guild").await?;
	}
	Ok(())
}

async fn get_spells(tome: &GuildTome) -> anyhow::Result<SpellCollection> {
	sources::get_spells(&tome.source).await
}
