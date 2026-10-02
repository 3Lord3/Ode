use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// App settings. Language and theme are NOT configurable, they always follow the
/// system (default AdwStyleManager, system locale). Only cache options live here,
/// stored in XDG_CONFIG_HOME/genius/config.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub cache_ttl_secs: u64,
    pub cache_max_mb: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache_ttl_secs: 3600,
            cache_max_mb: 50,
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        let dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(dirs_home_config);
        dir.join("genius").join("config.json")
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
}

fn dirs_home_config() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let c = Config::default();
        let s = serde_json::to_string(&c).unwrap();
        let back: Config = serde_json::from_str(&s).unwrap();
        assert_eq!(back.cache_ttl_secs, 3600);
    }
}