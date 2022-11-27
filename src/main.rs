#![feature(let_chains, async_closure)]
#![deny(clippy::pedantic)]
#![allow(clippy::unreadable_literal, clippy::wildcard_imports)]

use commands::build_spell_map;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;

use data::{Spell, SpellCollection};
use diesel::prelude::*;
use dotenvy::dotenv;
use poise::serenity_prelude as serenity;

mod commands;
mod data;

mod models;
mod schema;

pub struct Data {
	db: Arc<Mutex<MysqlConnection>>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

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
				prefix: Some(env::var("PREFIX").unwrap_or_else(|_| "//".to_string())),
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

				log::info!("Building spell info...");
				let guilds = ctx.cache.guilds();
				for guild in guilds {
					log::info!("For: {guild}");
					build_spell_map(guild, connection.clone(), false).await;
				}
				log::info!("Done");
				Ok(Data {
					db: connection.clone(),
				})
			})
		});

	framework.run().await.unwrap();
}
