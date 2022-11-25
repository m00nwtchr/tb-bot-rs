use crate::schema::*;
use diesel::prelude::*;

#[derive(Debug, Queryable)]
pub struct GuildTome {
	pub id: u32,
	pub guild: u64,
	pub source: String,
}

#[derive(Insertable)]
#[diesel(table_name = GuildTomes)]
pub struct NewGuildTome<'a> {
	pub guild: u64,
	pub source: &'a str,
}
