use anyhow::{Context, Result};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CONFIG_FILE: &str = ".cawa_cfg.json";
// kept separate from the config so committing the config doesn't leak run timestamps
const STATE_FILE: &str = ".cawa_state.json";

fn global_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("cawa").join("config.json")
}

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
    pub timeout_secs: Option<u64>,
}

impl Serialize for AliasConfig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // use the object form only when there are extra fields to store
        if self.description.is_some() || self.timeout_secs.is_some() {
            let mut map = serializer.serialize_map(None)?;
            match &self.entry {
                AliasEntry::Single(cmd) => map.serialize_entry("run", cmd)?,
                AliasEntry::Parallel(cmds) => map.serialize_entry("parallel", cmds)?,
            }
            if let Some(desc) = &self.description {
                map.serialize_entry("description", desc)?;
            }
            if let Some(t) = self.timeout_secs {
                map.serialize_entry("timeout_secs", &t)?;
            }
            map.end()
        } else {
            // no extra fields, keep the old bare format so existing configs don't change
            self.entry.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for AliasConfig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let val = Value::deserialize(deserializer)?;
        match &val {
            // old format: bare string or array — wrap with no extra fields
            Value::String(_) | Value::Array(_) => {
                let entry: AliasEntry =
                    serde_json::from_value(val).map_err(serde::de::Error::custom)?;
                Ok(AliasConfig { entry, description: None, timeout_secs: None })
            }
            // new format: { "run": "...", "description": "...", "timeout_secs": 60 }
            Value::Object(obj) => {
                let description = obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let timeout_secs = obj.get("timeout_secs").and_then(|v| v.as_u64());

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

                Ok(AliasConfig { entry, description, timeout_secs })
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

pub fn load_global_config() -> Result<Config> {
    let path = global_config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(&path)?;
    serde_json::from_str(&content).context("Failed to parse global config file")
}

pub fn save_global_config(config: &Config) -> Result<()> {
    let path = global_config_path();
    // create ~/.config/cawa/ if it doesn't exist yet
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(config)?;
    fs::write(&path, content).context("Failed to write global config file")
}

pub fn load_merged_config() -> Result<Config> {
    // start with global aliases, then overlay local ones so local always wins
    let mut merged = load_global_config().unwrap_or_default();
    let local = load_config()?;
    for (k, v) in local.aliases {
        merged.aliases.insert(k, v);
    }
    if local.identifier.is_some() {
        merged.identifier = local.identifier;
    }
    if local.enable_timing.is_some() {
        merged.enable_timing = local.enable_timing;
    }
    Ok(merged)
}

// last-run timestamps live in a separate file so they don't pollute the committed config
pub fn load_state() -> HashMap<String, u64> {
    fs::read_to_string(STATE_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_state(state: &HashMap<String, u64>) -> Result<()> {
    let content = serde_json::to_string_pretty(state)?;
    fs::write(STATE_FILE, content).context("Failed to write state file")
}

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
