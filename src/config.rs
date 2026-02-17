use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const CONFIG_FILE: &str = ".cawa_cfg.json";

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AliasEntry {
    Single(String),
    Parallel(Vec<String>),
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(default)]
    pub enable_timing: Option<bool>,
    #[serde(default)]
    pub aliases: HashMap<String, AliasEntry>,
}

pub fn load_config() -> Result<Config> {
    if !Path::new(CONFIG_FILE).exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(CONFIG_FILE)?;
    serde_json::from_str(&content).context("Failed to parse config file")
}

pub fn save_config(config: &Config) -> Result<()> {
    let content = serde_json::to_string_pretty(config)?;
    fs::write(CONFIG_FILE, content).context("Failed to write config file")
}
