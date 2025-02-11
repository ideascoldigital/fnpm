use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, anyhow};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    package_manager: String,
}

impl Config {
    pub fn new(package_manager: String) -> Self {
        Self { package_manager }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        let config_dir = config_path.parent().ok_or_else(|| anyhow!("Invalid config path"))?;
        fs::create_dir_all(config_dir)?;
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())    
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        if !config_path.exists() {
            return Err(anyhow!("No configuration found. Run 'fnpm setup' first"));
        }

        let content = fs::read_to_string(config_path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }

    fn get_config_path() -> Result<PathBuf> {
        let mut path = PathBuf::from(".");
        path.push(".fnpm");
        path.push("config.json");
        Ok(path)
    }

    pub fn get_package_manager(&self) -> &str {
        &self.package_manager
    }
}