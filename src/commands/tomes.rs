use diesel::prelude::*;
use poise::serenity_prelude as serenity;

use super::is_manager;
use crate::{models::*, Context, Error};

/// Manage sources of homebrew spells. (Avrae tomes, etc.)
#[poise::command(
	prefix_command,
	slash_command,
	guild_only,
	subcommands("list_tomes", "add_tome", "remove_tome")
)]
#[allow(clippy::unused_async)]
pub async fn tomes(_ctx: Context<'_>) -> Result<(), Error> {
	Ok(())
}

/// List tomes for this guild.
#[poise::command(prefix_command, slash_command, guild_only, rename = "list")]
async fn list_tomes(ctx: Context<'_>) -> Result<(), Error> {
	use crate::schema::GuildTomes::dsl::{guild, GuildTomes};

	let mut conn = ctx.data().db.lock().await;
	let serenity::GuildId(guild_id) = ctx.guild_id().expect("Guild Id");

	let tomes = GuildTomes
		.filter(guild.eq(guild_id))
		.load::<GuildTome>(&mut *conn)
		.expect("Error loading guild tomes.");

	let str = tomes
		.into_iter()
		.fold(String::new(), |prev, val| format!("{prev}{}\n", val.source));

	ctx.say(str).await?;

	Ok(())
}

/// Add tome for this guild.
#[poise::command(
	prefix_command,
	slash_command,
	guild_only,
	rename = "add",
	check = "is_manager"
)]
async fn add_tome(
	ctx: Context<'_>,
	#[description = "The tome to add (Avrae tome id)"] src: String,
) -> Result<(), Error> {
	use crate::schema::GuildTomes::dsl::*;

	let mut conn = ctx.data().db.lock().await;

	let serenity::GuildId(guild_id) = ctx.guild_id().expect("Guild Id");

	let count = GuildTomes
		.filter(guild.eq(guild_id))
		.filter(source.eq(&src))
		.count()
		.get_result::<i64>(&mut *conn)?;

	if count == 0 {
		let tome = NewGuildTome {
			guild: guild_id,
			source: &src,
		};

		diesel::insert_into(GuildTomes)
			.values(&tome)
			.execute(&mut *conn)?;

		ctx.say(format!("Successfully added: {src}")).await?;
	} else {
		ctx.say("This tome is already added for this guild.")
			.await?;
	}

	Ok(())
}

/// Remove tome from this guild.
#[poise::command(
	prefix_command,
	slash_command,
	guild_only,
	rename = "remove",
	check = "is_manager"
)]
async fn remove_tome(
	ctx: Context<'_>,
	#[description = "The tome to remove (Avrae tome id)"] src: String,
) -> Result<(), Error> {
	use crate::schema::GuildTomes::dsl::*;

	let mut conn = ctx.data().db.lock().await;
	let serenity::GuildId(guild_id) = ctx.guild_id().expect("Guild Id");

	let count = diesel::delete(
		GuildTomes
			.filter(guild.eq(guild_id))
			.filter(source.eq(&src)),
	)
	.execute(&mut *conn)?;

	if count > 0 {
		ctx.say(format!("Successfully removed: {src}")).await?;
	} else {
		ctx.say(format!("{src} is not added for this guild."))
			.await?;
	}

	Ok(())
}
