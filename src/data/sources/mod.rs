use lazy_static::lazy_static;
use regex::Regex;

use super::SpellCollection;

mod avrae;
mod fiveetools;
mod json;

pub use fiveetools::get_index as get_5e_index;

lazy_static! {
	static ref REGEX: Regex = Regex::new(r".*\d.*").unwrap();
}

pub async fn get_spells(source: &str) -> anyhow::Result<SpellCollection> {
	// log::info!("Get spells from source: {source}");
	if let Ok(url) = reqwest::Url::parse(source) {
		json::get_tome(url).await.map(Into::into)
	} else if REGEX.is_match(source) || source.eq("srd") {
		avrae::get_tome(source).await.map(Into::into)
	} else {
		fiveetools::get_source(source).await.map(Into::into)
	}
}
