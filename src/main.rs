#![feature(let_chains, async_closure)]
#![deny(clippy::pedantic)]
#![allow(clippy::wildcard_imports)]

use std::fmt::Debug;
use std::sync::Arc;
use std::{collections::HashMap, env};

use diesel::prelude::*;
use dotenvy::dotenv;
use poise::serenity_prelude::{self as serenity, CacheHttp, GuildId};
use tokio::sync::{Mutex, RwLock};

use commands::build_spell_map;
use commands::spells::SpellMap;

mod commands;
mod data;

mod models;
mod schema;

pub struct Data {
	db: Arc<Mutex<MysqlConnection>>,
	spell_map: Arc<RwLock<HashMap<GuildId, SpellMap>>>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

impl Debug for Data {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("")
	}
}

#[tokio::main]
async fn main() {
	dotenv().ok();
	env_logger::init();

	let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
	let connection = Arc::new(Mutex::new(loop {
		match MysqlConnection::establish(&db_url) {
			Ok(conn) => break conn,
			Err(e) => log::error!("Error connecting to {db_url}: {e}"),
		}
	}));

	let framework = poise::Framework::builder()
		.options(poise::FrameworkOptions {
			commands: commands::commands(),
			prefix_options: poise::PrefixFrameworkOptions {
				prefix: Some(env::var("PREFIX").unwrap_or_else(|_| "!!".to_string())),
				..Default::default()
			},
			..Default::default()
		})
		.token(env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
		.intents(
			serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT,
		)
		.setup(|ctx, _ready, framework| {
			Box::pin(async move {
				poise::builtins::register_globally(ctx, &framework.options().commands).await?;
				let mut spell_map = HashMap::new();

				log::info!("Building spell lists...");
				let guilds = ctx.cache.guilds();
				for guild_id in guilds {
					log::info!("For: {:?} - {guild_id}", guild_id.name(&ctx.cache));

					spell_map.insert(
						guild_id,
						build_spell_map(guild_id, connection.clone()).await,
					);
				}
				log::info!("Done");

				Ok(Data {
					db: connection.clone(),
					spell_map: Arc::new(RwLock::new(spell_map)),
				})
			})
		});

	framework.run().await.unwrap();
}
