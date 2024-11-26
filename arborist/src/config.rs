use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub db_url: String,
    pub collection_name: String,
    pub scan: ScanConfig,
    pub query: QueryConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_url: "http://localhost:6334".to_string(),
            collection_name: "file_data".to_string(),
            scan: ScanConfig::default(),
            query: QueryConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScanConfig {
    pub max_tokens: (usize, usize),
    pub model_name: String,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            max_tokens: (20, 40),
            model_name: "gemma2:2b".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryConfig {
    pub top_k_results: usize,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self { top_k_results: 5 }
    }
}

impl Config {
    /// Load the configuration from the given path or fallback to defaults
    pub fn load(config_path: Option<PathBuf>) -> anyhow::Result<Self> {
        if let Some(path) = config_path {
            // Load user-provided config file
            let contents = fs::read_to_string(path)?;
            let config: Config = toml::from_str(&contents)?;
            Ok(config)
        } else {
            // Look for default config file
            let default_path = dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("arborist/config.toml");

            if default_path.exists() {
                let contents = fs::read_to_string(&default_path)?;
                let config: Config = toml::from_str(&contents)?;
                Ok(config)
            } else {
                // Generate a new default config
                let default_config = Config::default();

                fs::create_dir_all(default_path.parent().unwrap())?;
                fs::write(&default_path, toml::to_string_pretty(&default_config)?)?;
                println!("Default config file created at: {}", default_path.display());
                Ok(default_config)
            }
        }
    }
}
