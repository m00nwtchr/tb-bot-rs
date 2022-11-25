use lazy_static::lazy_static;
use regex::Regex;

use super::SpellCollection;

mod avrae;
mod fiveetools;
mod json;

lazy_static! {
	static ref REGEX: Regex = Regex::new(r".*\d.*").unwrap();
}

pub async fn get_spells(source: &str) -> anyhow::Result<SpellCollection> {
	if let Ok(url) = reqwest::Url::parse(source) {
		json::get_json(url).await.map(Into::into)
	} else if REGEX.is_match(source) {
		avrae::get_tome(source).await.map(Into::into)
	} else if source.eq("srd") {
		avrae::get_srd().await.map(Into::into)
	} else {
		fiveetools::get_source(source).await.map(Into::into)
	}
}
