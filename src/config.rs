use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    package_manager: String,
    pub global_cache_path: String,
    /// The lockfile that should be kept updated (e.g., "pnpm-lock.yaml" when using yarn but project uses pnpm)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_lockfile: Option<String>,
    /// Enable security auditing for package installations
    #[serde(default = "default_security_audit")]
    pub security_audit: bool,
    /// Maximum depth for transitive dependency scanning (0 = disabled, 1-5 = depth)
    #[serde(default = "default_transitive_scan_depth")]
    pub transitive_scan_depth: usize,
    /// Minimum age (in minutes) a package version must have on the registry before
    /// it can be installed. Mirrors pnpm's `minimumReleaseAge`. Default: 1440 (1 day).
    /// Set to 0 to disable.
    #[serde(default = "default_minimum_release_age_minutes")]
    pub minimum_release_age_minutes: u64,
    /// Reject top-level dependencies whose specifier is not a normal semver/dist-tag
    /// range (git, http, file, github:, etc.). Mirrors pnpm's `blockExoticSubdeps`.
    #[serde(default = "default_block_exotic_subdeps")]
    pub block_exotic_subdeps: bool,
    /// Allowlist of packages whose lifecycle scripts (preinstall/install/postinstall)
    /// fnpm is permitted to execute. Mirrors pnpm's `allowBuilds`. Empty = block all.
    #[serde(default)]
    pub allow_builds: Vec<String>,
}

fn default_security_audit() -> bool {
    true
}

fn default_transitive_scan_depth() -> usize {
    2
}

fn default_minimum_release_age_minutes() -> u64 {
    1440
}

fn default_block_exotic_subdeps() -> bool {
    true
}

fn default_global_cache_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{}/{}/.fnpm/cache", home, ".local/share")
}

impl Config {
    pub fn new(package_manager: String) -> Self {
        Self {
            package_manager,
            global_cache_path: default_global_cache_path(),
            target_lockfile: None,
            security_audit: default_security_audit(),
            transitive_scan_depth: default_transitive_scan_depth(),
            minimum_release_age_minutes: default_minimum_release_age_minutes(),
            block_exotic_subdeps: default_block_exotic_subdeps(),
            allow_builds: Vec::new(),
        }
    }

    pub fn new_with_lockfile(package_manager: String, target_lockfile: Option<String>) -> Self {
        Self {
            package_manager,
            global_cache_path: default_global_cache_path(),
            target_lockfile,
            security_audit: default_security_audit(),
            transitive_scan_depth: default_transitive_scan_depth(),
            minimum_release_age_minutes: default_minimum_release_age_minutes(),
            block_exotic_subdeps: default_block_exotic_subdeps(),
            allow_builds: Vec::new(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        let config_dir = config_path
            .parent()
            .ok_or_else(|| anyhow!("Invalid config path"))?;
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

    pub fn get_target_lockfile(&self) -> Option<&str> {
        self.target_lockfile.as_deref()
    }

    pub fn set_target_lockfile(&mut self, lockfile: Option<String>) {
        self.target_lockfile = lockfile;
    }

    pub fn is_security_audit_enabled(&self) -> bool {
        self.security_audit
    }

    pub fn set_security_audit(&mut self, enabled: bool) {
        self.security_audit = enabled;
    }

    pub fn get_transitive_scan_depth(&self) -> usize {
        self.transitive_scan_depth.min(5) // Max 5 levels
    }

    pub fn set_transitive_scan_depth(&mut self, depth: usize) {
        self.transitive_scan_depth = depth.min(5);
    }

    pub fn get_minimum_release_age_minutes(&self) -> u64 {
        self.minimum_release_age_minutes
    }

    pub fn is_block_exotic_subdeps(&self) -> bool {
        self.block_exotic_subdeps
    }

    pub fn get_allow_builds(&self) -> &[String] {
        &self.allow_builds
    }

    /// Load config from `.fnpm/config.json`, or fall back to defaults if none exists.
    /// Use this in security paths so protections apply even before `fnpm setup`.
    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_else(|_| Self::new(String::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        env::set_current_dir(temp_dir.path()).expect("Failed to change dir");
        temp_dir
    }

    #[test]
    fn test_config_new() {
        let config = Config::new("npm".to_string());
        assert_eq!(config.get_package_manager(), "npm");
        assert!(config.global_cache_path.contains(".fnpm/cache"));
    }

    #[test]
    #[serial_test::serial]
    fn test_config_save_and_load() {
        let temp_dir = setup_test_env();

        let original_config = Config::new("yarn".to_string());

        // Test save (this will create the .fnpm directory)
        original_config.save().expect("Failed to save config");

        // Verify config file was created
        let config_path = temp_dir.path().join(".fnpm").join("config.json");
        assert!(config_path.exists());

        // Test load
        let loaded_config = Config::load().expect("Failed to load config");
        assert_eq!(loaded_config.get_package_manager(), "yarn");
        assert_eq!(
            loaded_config.global_cache_path,
            original_config.global_cache_path
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_config_load_nonexistent() {
        let _temp_dir = setup_test_env();

        let result = Config::load();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No configuration found"));
    }

    #[test]
    fn test_default_global_cache_path() {
        let path = default_global_cache_path();
        assert!(path.contains(".fnpm/cache"));
        assert!(path.contains(".local/share"));
    }

    #[test]
    fn test_get_config_path() {
        let path = Config::get_config_path().expect("Failed to get config path");
        assert!(path.to_string_lossy().contains(".fnpm"));
        assert!(path.to_string_lossy().contains("config.json"));
    }
}
