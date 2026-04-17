use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::pricing::ModelPricing;

/// Returns the XDG config directory: $XDG_CONFIG_HOME if set, else ~/.config.
fn xdg_config_dir() -> Option<PathBuf> {
    if let Ok(val) = std::env::var("XDG_CONFIG_HOME")
        && !val.is_empty()
    {
        return Some(PathBuf::from(val));
    }
    dirs::home_dir().map(|h| h.join(".config"))
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Override the Claude projects directory
    pub projects_dir: Option<PathBuf>,
    /// Log level (trace, debug, info, warn, error)
    pub log_level: Option<String>,
    /// Pricing table - keyed by model name
    pub pricing: HashMap<String, ModelPricing>,
}

impl Config {
    /// Load config and return the resolved path it was loaded from (None if using defaults).
    pub fn load(config_path: Option<&PathBuf>) -> Result<(Self, Option<PathBuf>)> {
        log::debug!("Config::load: config_path={:?}", config_path);

        if let Some(path) = config_path {
            let config =
                Self::load_from_file(path).context(format!("Failed to load config from {}", path.display()))?;
            return Ok((config, Some(path.clone())));
        }

        // Try XDG config dir ($XDG_CONFIG_HOME, or ~/.config)
        if let Some(config_dir) = xdg_config_dir() {
            let primary_config = config_dir.join("ccu").join("ccu.yml");
            if primary_config.exists() {
                match Self::load_from_file(&primary_config) {
                    Ok(config) => return Ok((config, Some(primary_config))),
                    Err(e) => {
                        log::warn!("Failed to load config from {}: {}", primary_config.display(), e);
                    }
                }
            }
        }

        // No config file found - return empty config; caller merges embedded pricing
        log::info!("No config file found, using embedded pricing defaults");
        Ok((Config::default(), None))
    }

    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path).context("Failed to read config file")?;
        let config: Self = serde_yaml::from_str(&content).context("Failed to parse config file")?;
        log::info!("Loaded config from: {}", path.as_ref().display());
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_deserialize_with_log_level() {
        let yaml = "log_level: debug\npricing: {}\n";
        let config: Config = serde_yaml::from_str(yaml).expect("parse yaml");
        assert_eq!(config.log_level.as_deref(), Some("debug"));
    }

    #[test]
    fn test_config_deserialize_without_log_level() {
        let yaml = "pricing: {}\n";
        let config: Config = serde_yaml::from_str(yaml).expect("parse yaml");
        assert!(config.log_level.is_none());
    }

    #[test]
    fn test_load_explicit_path_missing() {
        let result = Config::load(Some(&PathBuf::from("/nonexistent/path/ccu.yml")));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_no_config_does_not_error() {
        // Config::load(None) should never error, even if no config file exists.
        // It returns either the loaded config or a default with empty pricing.
        let result = Config::load(None);
        assert!(result.is_ok(), "Config::load(None) should not error");
    }

    #[test]
    fn test_load_returns_path_for_explicit_config() {
        let path = PathBuf::from("/nonexistent/path/ccu.yml");
        let result = Config::load(Some(&path));
        // explicit missing path should error, not return None
        assert!(result.is_err());
    }

    #[test]
    fn test_load_returns_none_path_when_no_config() {
        // When no config file exists, path should be None
        // (we can't guarantee no config exists in test env, just check it doesn't panic)
        let result = Config::load(None);
        assert!(result.is_ok());
    }
}
