//! Configuration loading from bosun.toml.

use policy::Policy;
use serde::Deserialize;
use std::path::Path;

/// Top-level configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Backend configuration.
    #[serde(default)]
    pub backend: BackendConfig,

    /// Policy rules (allow/deny).
    #[serde(flatten)]
    pub policy: Policy,
}

/// Backend provider configuration.
#[derive(Debug, Deserialize)]
pub struct BackendConfig {
    /// Provider name (currently only "anthropic" supported).
    /// Reserved for future multi-provider support.
    #[serde(default = "default_provider")]
    #[allow(dead_code)]
    pub provider: String,

    /// Model to use.
    #[serde(default = "default_model")]
    pub model: String,

    /// API key (if not set, reads from ANTHROPIC_API_KEY env var).
    pub api_key: Option<String>,
}

fn default_provider() -> String {
    "anthropic".to_string()
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            api_key: None,
        }
    }
}

impl Config {
    /// Load configuration from a TOML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path.as_ref())?;
        Self::parse(&content)
    }

    /// Parse configuration from TOML string.
    pub fn parse(toml: &str) -> Result<Self, ConfigError> {
        toml::from_str(toml).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// Create a default configuration.
    pub fn default_config() -> Self {
        Self {
            backend: BackendConfig::default(),
            policy: Policy::restrictive(),
        }
    }

    /// Get the API key, falling back to environment variable.
    pub fn api_key(&self) -> Result<String, ConfigError> {
        if let Some(key) = &self.backend.api_key {
            return Ok(key.clone());
        }

        std::env::var("ANTHROPIC_API_KEY").map_err(|_| ConfigError::MissingApiKey)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse config: {0}")]
    Parse(String),

    #[error("API key not configured (set backend.api_key in config or ANTHROPIC_API_KEY env var)")]
    MissingApiKey,
}
