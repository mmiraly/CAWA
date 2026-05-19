use anyhow::{Context, Result};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
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

// wraps an alias entry with an optional description
// supports both the old bare format and the new object format
#[derive(Clone)]
pub struct AliasConfig {
    pub entry: AliasEntry,
    pub description: Option<String>,
}

impl Serialize for AliasConfig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if let Some(desc) = &self.description {
            // only use the object form when there's actually a description
            let mut map = serializer.serialize_map(None)?;
            match &self.entry {
                AliasEntry::Single(cmd) => map.serialize_entry("run", cmd)?,
                AliasEntry::Parallel(cmds) => map.serialize_entry("parallel", cmds)?,
            }
            map.serialize_entry("description", desc)?;
            map.end()
        } else {
            // no description, keep the old bare format so existing configs don't change
            self.entry.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for AliasConfig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let val = Value::deserialize(deserializer)?;
        match &val {
            // old format: bare string or array — wrap with no description
            Value::String(_) | Value::Array(_) => {
                let entry: AliasEntry =
                    serde_json::from_value(val).map_err(serde::de::Error::custom)?;
                Ok(AliasConfig { entry, description: None })
            }
            // new format: { "run": "...", "description": "..." }
            Value::Object(obj) => {
                let description = obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let entry = if let Some(run) = obj.get("run") {
                    let cmd: String =
                        serde_json::from_value(run.clone()).map_err(serde::de::Error::custom)?;
                    AliasEntry::Single(cmd)
                } else if let Some(parallel) = obj.get("parallel") {
                    let cmds: Vec<String> = serde_json::from_value(parallel.clone())
                        .map_err(serde::de::Error::custom)?;
                    AliasEntry::Parallel(cmds)
                } else {
                    return Err(serde::de::Error::custom(
                        "alias config must have a 'run' or 'parallel' field",
                    ));
                };

                Ok(AliasConfig { entry, description })
            }
            _ => Err(serde::de::Error::custom("invalid alias format")),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(default)]
    pub enable_timing: Option<bool>,
    #[serde(default)]
    pub aliases: HashMap<String, AliasConfig>,
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
