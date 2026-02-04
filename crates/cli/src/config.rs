//! Configuration loading from bosun.toml.

use policy::Policy;
use runtime::AnthropicAuth;
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
#[derive(Debug, Deserialize, Default)]
pub struct BackendConfig {
    /// Provider name (currently only "anthropic" supported).
    #[serde(default = "default_provider")]
    #[allow(dead_code)]
    pub provider: String,

    /// Model to use.
    #[serde(default = "default_model")]
    pub model: String,

    /// Standard Anthropic API key (sk-ant-api01-...).
    /// Mutually exclusive with oauth_token.
    pub api_key: Option<String>,

    /// Claude Code OAuth token (sk-ant-oat-...).
    /// Mutually exclusive with api_key.
    pub oauth_token: Option<String>,
}

fn default_provider() -> String {
    "anthropic".to_string()
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
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

    /// Build the authentication from config.
    ///
    /// Requires exactly one of api_key or oauth_token to be set.
    pub fn auth(&self) -> Result<AnthropicAuth, ConfigError> {
        match (&self.backend.api_key, &self.backend.oauth_token) {
            (Some(key), None) => Ok(AnthropicAuth::ApiKey(key.clone())),
            (None, Some(token)) => Ok(AnthropicAuth::ClaudeCodeOauth(token.clone())),
            (Some(_), Some(_)) => Err(ConfigError::AmbiguousAuth),
            (None, None) => Err(ConfigError::MissingAuth),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse config: {0}")]
    Parse(String),

    #[error("authentication not configured: set backend.api_key or backend.oauth_token")]
    MissingAuth,

    #[error(
        "ambiguous authentication: set either backend.api_key OR backend.oauth_token, not both"
    )]
    AmbiguousAuth,
}
