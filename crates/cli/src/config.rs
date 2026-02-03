//! Configuration loading for bosun.

use mcp::ServerConfig;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};

/// Full bosun configuration.
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    /// Policy section (handled separately by policy crate).
    /// These fields exist to allow the TOML to contain policy config
    /// without causing parse errors.
    #[serde(default)]
    #[allow(dead_code)]
    allow: Option<toml::Value>,
    #[serde(default)]
    #[allow(dead_code)]
    deny: Option<toml::Value>,

    /// MCP server configurations.
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
}

/// MCP server configuration from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl From<McpServerConfig> for ServerConfig {
    fn from(cfg: McpServerConfig) -> Self {
        Self {
            name: cfg.name,
            command: cfg.command,
            args: cfg.args,
            env: cfg.env,
        }
    }
}

impl Config {
    /// Load config from a TOML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        Self::parse(&content)
    }

    /// Parse config from TOML string.
    pub fn parse(toml: &str) -> Result<Self> {
        toml::from_str(toml).map_err(|e| Error::Config(e.to_string()))
    }

    /// Get MCP server configs.
    pub fn mcp_servers(&self) -> Vec<ServerConfig> {
        self.mcp_servers.iter().cloned().map(Into::into).collect()
    }
}
