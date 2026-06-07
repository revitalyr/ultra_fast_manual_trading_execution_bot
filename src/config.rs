use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchConfig {
    pub id: String,
    pub name: String,
    pub goal_market_id: String,
    pub match_market_id: String,
    pub max_price_limit: f64,
    #[serde(default)]
    pub keyboard_shortcut: Option<char>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub matches: Vec<MatchConfig>,
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn from_default_path() -> Result<Self> {
        let config_path = std::env::var("CONFIG_PATH")
            .unwrap_or_else(|_| "config.toml".to_string());
        Self::from_file(config_path)
    }

    pub fn get_match_configs(&self) -> Vec<crate::match_engine::MatchConfig> {
        self.matches
            .iter()
            .map(|m| crate::match_engine::MatchConfig::new(
                m.id.clone(),
                m.name.clone(),
                m.goal_market_id.clone(),
                m.match_market_id.clone(),
                m.max_price_limit,
                m.keyboard_shortcut,
            ))
            .collect()
    }
}
