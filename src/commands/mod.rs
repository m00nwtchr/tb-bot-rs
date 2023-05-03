use convert_case::Casing;
use futures::{Stream, StreamExt};
use poise::serenity_prelude::{
	self as serenity, CacheHttp, CreateComponents, CreateEmbed, ReactionType,
};
use std::{
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
	time::Duration,
};

use crate::{data::SpellSchool, Context, Error};

pub mod spells;
mod tomes;

pub use spells::build_spell_map;

/// Show this menu
#[poise::command(prefix_command, track_edits, slash_command)]
async fn help(
	ctx: Context<'_>,
	#[description = "Specific command to show help about"] command: Option<String>,
) -> Result<(), Error> {
	// let config = poise::builtins::HelpConfiguration {
	// 		extra_text_at_bottom: "\
	// Type !!help command for more info on a command.
	// You can edit your message to the bot and the bot will edit its response.",
	// 	..Default::default()
	// };
	poise::builtins::help(ctx, command.as_deref(), Default::default()).await?;
	Ok(())
}

async fn autocomplete_class<'a>(
	ctx: Context<'_>,
	partial: &'a str,
) -> impl Stream<Item = String> + 'a {
	let spell_map = ctx.data().spell_map.read().await;

	let id = ctx.guild_id().unwrap_or_default();
	let vec = spell_map
		.get(&id)
		.map_or(Vec::new(), |sm| sm.get_classes().clone());

	futures::stream::iter(vec)
		.filter(move |class| futures::future::ready(class.starts_with(partial)))
		// .map(String::clone)
		.map(|class| class.to_case(convert_case::Case::Title))
}

#[allow(clippy::unused_async)]
async fn autocomplete_level(
	_ctx: Context<'_>,
	_partial: &str,
) -> impl Iterator<Item = poise::AutocompleteChoice<u8>> {
	(1u8..9).map(|n| poise::AutocompleteChoice {
		name: format!("{n}"),
		value: n,
	})
}

#[allow(clippy::unused_async)]
async fn autocomplete_school(_ctx: Context<'_>, _partial: &str) -> [SpellSchool; 8] {
	SpellSchool::all()
}

pub fn commands() -> Vec<poise::Command<crate::Data, crate::Error>> {
	vec![
		help(),
		tomes::tomes(),
		spells::spell_list_slash(),
		spells::spell_list_prefix(),
		spells::rebuild(),
	]
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

pub async fn send_paginated_message(
	ctx: Context<'_>,
	pages: Vec<String>,
	mut embed: CreateEmbed,
) -> Result<(), Error> {
	fn mk_btns<'a>(
		c: &'a mut CreateComponents,
		len: usize,
		page_index: &Arc<AtomicUsize>,
	) -> &'a mut CreateComponents {
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

	let page_index = Arc::new(AtomicUsize::new(0));
	let pages = Arc::new(pages);

	embed
		.description(pages.get(0).unwrap())
		.footer(|f| f.text(format!("Page {} out of {}", 1, pages.len())));

	let reply = ctx
		.send(|m| {
			m.embeds.push(embed.clone());
			if pages.len() > 1 {
				m.components(|c| mk_btns(c, pages.len(), &page_index));
			}
			m.ephemeral(true)
		})
		.await?;

	let mut interactions = reply
		.message()
		.await?
		.await_component_interactions(ctx)
		.timeout(Duration::from_secs(10 * 60))
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
						let mut components = CreateComponents::default();
						mk_btns(&mut components, pages.len(), &page_index);

						d.set_embed(embed)
							.set_components(components)
							.ephemeral(true)
					})
			})
			.await?;
	}

	Ok(())
}
