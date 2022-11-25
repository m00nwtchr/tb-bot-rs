use std::{
	ascii::AsciiExt,
	collections::HashMap,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	}, time::Duration,
};

use crate::{
	data::{sources, Spell, SpellCollection, SpellSchool},
	models::GuildTome,
	Context, Error,
};
use convert_case::Casing;
use diesel::prelude::*;
use futures::{stream::FuturesUnordered, Stream, StreamExt};
use itertools::Itertools;
use lazy_static::lazy_static;
use poise::serenity_prelude::{
	self as serenity, CacheHttp, CreateComponents, CreateEmbed, GuildId, ReactionType,
};
use tokio::sync::Mutex;
// use tokio_stream::{self as stream, StreamExt};

mod tomes;

/// Show this menu
#[poise::command(prefix_command, track_edits, slash_command)]
async fn help(
	ctx: Context<'_>,
	#[description = "Specific command to show help about"] command: Option<String>,
) -> Result<(), Error> {
	let config = poise::builtins::HelpConfiguration {
		// 		extra_text_at_bottom: "\
		// Type !!help command for more info on a command.
		// You can edit your message to the bot and the bot will edit its response.",
		..Default::default()
	};
	poise::builtins::help(ctx, command.as_deref(), config).await?;
	Ok(())
}

lazy_static! {
	static ref SPELL_MAP: Arc<Mutex<HashMap<GuildId, SpellMap>>> =
		Arc::new(Mutex::new(HashMap::new()));
}

async fn autocomplete_class<'a>(ctx: Context<'a>, partial: &'a str) -> Vec<String> {
	let spell_map = SPELL_MAP.lock().await;
	if let Some(spell_map) = ctx.guild_id().and_then(|id| spell_map.get(&id)) {
		spell_map
			.map
			.keys()
			.into_iter()
			.filter(move |class| class.starts_with(partial))
			.cloned()
			.map(|class| class.to_case(convert_case::Case::Title))
			.collect()
	} else {
		Vec::new()
	}
}

/// Lists spells for specified class and level
#[poise::command(
	slash_command,
	prefix_command,
	ephemeral,
	rename = "sl",
	aliases("spells")
)]
async fn spell_list(
	ctx: Context<'_>,
	#[autocomplete = "autocomplete_class"]
	#[description = "Class"]
	class: String,
	#[description = "Spell level"] level: Option<String>,
	#[description = "Filter spell schools (--<spell school> in additional args)"]
	spell_schools: Vec<SpellSchool>,
	#[description = "Only display ritual spells"]
	#[flag]
	ritual: bool,
	#[autocomplete = "autocomplete_class"]
	#[description = "Exclude spells which belong to this class's spell list (--!<class> in additional args)"]
	not_classes: Vec<String>,
	// #[rest]
	// #[description = "Additional arguments"]
	// args: Option<String>,
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

	send_paginated_message(ctx, &list, CreateEmbed::default()).await?;

	Ok(())
}

pub fn commands() -> Vec<poise::Command<crate::Data, crate::Error>> {
	vec![help(), tomes::tomes(), spell_list()]
}

pub async fn is_manager(ctx: Context<'_>) -> Result<bool, Error> {
	let author = ctx.author_member().await;

	if let Some(author) = author && let Some(cache) = ctx.cache() {
		let role = author.roles(cache)
			.and_then(|roles| roles.into_iter().find(|role| role.name.eq("Server Brewer")))
			.is_some();

		let res = role || author.permissions(cache)?.manage_guild() || author.user.id.eq(&302379230308859905u64);

		if !res {
			ctx.say("You don't have permission to use this command.").await?;
		}
		Ok(res)
	} else {
		Ok(false)
	}
}

#[derive(Debug, Clone, Default)]
pub struct SpellMap {
	spells: Vec<Spell>,
	map: HashMap<String, Vec<usize>>,
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

pub async fn build_spell_map(guild_id: GuildId, conn: Arc<Mutex<MysqlConnection>>, rebuild: bool) -> SpellMap {
	use super::schema::GuildTomes::dsl::*;

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

	let mut sm = SpellMap::default();
	let mut spells_futures: FuturesUnordered<_> = tomes.iter().map(get_spells).collect();

	while let Some(coll) = spells_futures.next().await {
		if let Ok(coll) = coll {
			for spell in coll.spells {
				sm.add_spell(spell)
			}
		}
	}

	SPELL_MAP.lock().await.insert(guild_id, sm.clone());

	sm
}

async fn get_spells(tome: &GuildTome) -> anyhow::Result<SpellCollection> {
	sources::get_spells(&tome.source).await
}

pub async fn send_paginated_message(
	ctx: Context<'_>,
	pages: &Vec<String>,
	mut embed: CreateEmbed,
) -> Result<(), Error> {
	let page_index = Arc::new(AtomicUsize::new(0));
	let pages = Arc::new(pages.clone());

	embed
		.description(pages.get(0).unwrap())
		.footer(|f| f.text(format!("Page {} out of {}", 1, pages.len())));

	fn mk_btns(
		c: &mut CreateComponents,
		len: usize,
		page_index: Arc<AtomicUsize>,
	) -> &mut CreateComponents {
		c.create_action_row(|r| {
			let i = page_index.load(Ordering::Relaxed);
			if i > 0 {
				r.create_button(|b| {
					b.custom_id("interact.prev")
						.label("Previous")
						.style(serenity::ButtonStyle::Primary)
						.emoji(ReactionType::Unicode("⬅️".to_string()))
				});
			}
			if i < len - 1 {
				r.create_button(|b| {
					b.custom_id("interact.next")
						.label("Next")
						.style(serenity::ButtonStyle::Primary)
						.emoji(ReactionType::Unicode("➡️".to_string()))
				});
			}
			r
		})
	}

	let reply = ctx
		.send(|m| {
			m.embeds.push(embed.clone());
			if pages.len() > 1 {
				m.components(|c| mk_btns(c, pages.len(), page_index.clone()));
			}
			m.ephemeral(true)
		})
		.await?;

	let mut interactions = reply
		.message()
		.await?
		.await_component_interactions(ctx)
		.timeout(Duration::from_secs(10*60))
		.author_id(ctx.author().id)
		.build();

	while let Some(interaction) = interactions.next().await {
		let mut embed = embed.clone();
		let page_index = page_index.clone();
		let pages = pages.clone();

		let mut i = page_index.load(Ordering::Relaxed);
		match interaction.data.custom_id.as_str() {
			"interact.prev" => {
				i -= 1;
			}
			"interact.next" => {
				i += 1;
			}
			_ => {}
		}
		page_index.store(i, Ordering::Relaxed);

		if let Some(desc) = pages.get(i) {
			embed
				.description(desc)
				.footer(|f| f.text(format!("Page {} out of {}", i + 1, pages.len())));
		}

		interaction
			.create_interaction_response(ctx, |r| {
				r.kind(serenity::InteractionResponseType::UpdateMessage)
					.interaction_response_data(|d| {
						d.set_embed(embed);

						// if i > 0 || i < pages.len() - 1 {
						let mut components = CreateComponents::default();
						mk_btns(&mut components, pages.len(), page_index);

						d.set_components(components);
						// }

						d.ephemeral(true)
					})
			})
			.await?;
	}

	Ok(())
}
