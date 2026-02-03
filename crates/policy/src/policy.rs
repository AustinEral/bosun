//! Policy configuration and enforcement.

use crate::{CapabilityKind, CapabilityRequest, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// Policy configuration loaded from TOML.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Policy {
    /// Capabilities that are explicitly allowed.
    #[serde(default)]
    pub allow: AllowRules,

    /// Capabilities that are explicitly denied (overrides allow).
    #[serde(default)]
    pub deny: DenyRules,
}

/// Rules for allowed capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AllowRules {
    /// Allowed file read paths (glob patterns).
    #[serde(default)]
    pub fs_read: Vec<String>,

    /// Allowed file write paths (glob patterns).
    #[serde(default)]
    pub fs_write: Vec<String>,

    /// Allowed HTTP domains.
    #[serde(default)]
    pub net_http: Vec<String>,

    /// Allowed commands (exact or prefix match).
    #[serde(default)]
    pub exec: Vec<String>,

    /// Allowed secret keys.
    #[serde(default)]
    pub secrets_read: Vec<String>,
}

/// Rules for denied capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DenyRules {
    /// Deny all capabilities of these kinds.
    #[serde(default)]
    pub all: HashSet<CapabilityKind>,
}

/// Result of a capability check.
#[derive(Debug, Clone)]
pub enum Decision {
    Allow,
    Deny { reason: String },
}

impl Decision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Decision::Allow)
    }
}

impl Policy {
    /// Load policy from a TOML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        Self::parse(&content)
    }

    /// Parse policy from TOML string.
    pub fn parse(toml: &str) -> Result<Self> {
        toml::from_str(toml).map_err(|e| Error::Parse(e.to_string()))
    }

    /// Create a default restrictive policy (deny all side effects).
    pub fn restrictive() -> Self {
        let mut deny_all = HashSet::new();
        deny_all.insert(CapabilityKind::Exec);
        deny_all.insert(CapabilityKind::NetHttp);
        deny_all.insert(CapabilityKind::SecretsRead);

        Self {
            allow: AllowRules {
                fs_read: vec![".".to_string()],  // Current dir only
                fs_write: vec![".".to_string()], // Current dir only
                ..Default::default()
            },
            deny: DenyRules { all: deny_all },
        }
    }

    /// Check if a capability request is allowed.
    pub fn check(&self, request: &CapabilityRequest) -> Decision {
        // Check explicit denials first
        if self.deny.all.contains(&request.kind) {
            return Decision::Deny {
                reason: format!("{:?} is denied by policy", request.kind),
            };
        }

        // Check allowlist
        let allowed = match request.kind {
            CapabilityKind::FsRead => self.check_path_allowed(&self.allow.fs_read, &request.scope),
            CapabilityKind::FsWrite => self.check_path_allowed(&self.allow.fs_write, &request.scope),
            CapabilityKind::NetHttp => self.check_domain_allowed(&self.allow.net_http, &request.scope),
            CapabilityKind::Exec => self.check_command_allowed(&self.allow.exec, &request.scope),
            CapabilityKind::SecretsRead => self.check_exact_allowed(&self.allow.secrets_read, &request.scope),
        };

        if allowed {
            Decision::Allow
        } else {
            Decision::Deny {
                reason: format!(
                    "{:?} not in allowlist{}",
                    request.kind,
                    request.scope.as_ref().map(|s| format!(" (scope: {})", s)).unwrap_or_default()
                ),
            }
        }
    }

    fn check_path_allowed(&self, allowlist: &[String], scope: &Option<String>) -> bool {
        let Some(path) = scope else {
            return !allowlist.is_empty(); // No scope = any path, allow if list non-empty
        };

        for pattern in allowlist {
            if pattern == "*" || pattern == "**" {
                return true;
            }
            if path.starts_with(pattern) {
                return true;
            }
            // Simple glob: foo/* matches foo/bar but not foo/bar/baz
            if pattern.ends_with("/*") {
                let prefix = &pattern[..pattern.len() - 2];
                if path.starts_with(prefix) && !path[prefix.len()..].contains('/') {
                    return true;
                }
            }
            // Recursive glob: foo/** matches foo/bar/baz
            if pattern.ends_with("/**") {
                let prefix = &pattern[..pattern.len() - 3];
                if path.starts_with(prefix) {
                    return true;
                }
            }
        }
        false
    }

    fn check_domain_allowed(&self, allowlist: &[String], scope: &Option<String>) -> bool {
        let Some(domain) = scope else {
            return !allowlist.is_empty();
        };

        for allowed in allowlist {
            if allowed == "*" {
                return true;
            }
            if domain == allowed || domain.ends_with(&format!(".{}", allowed)) {
                return true;
            }
        }
        false
    }

    fn check_command_allowed(&self, allowlist: &[String], scope: &Option<String>) -> bool {
        let Some(cmd) = scope else {
            return !allowlist.is_empty();
        };

        for allowed in allowlist {
            if allowed == "*" {
                return true;
            }
            // Exact match or prefix match (e.g., "git" allows "git status")
            if cmd == allowed || cmd.starts_with(&format!("{} ", allowed)) {
                return true;
            }
        }
        false
    }

    fn check_exact_allowed(&self, allowlist: &[String], scope: &Option<String>) -> bool {
        let Some(key) = scope else {
            return !allowlist.is_empty();
        };

        allowlist.iter().any(|a| a == "*" || a == key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restrictive_denies_exec() {
        let policy = Policy::restrictive();
        let req = CapabilityRequest::exec("rm -rf /");
        assert!(!policy.check(&req).is_allowed());
    }

    #[test]
    fn test_allow_fs_read_in_workspace() {
        let policy = Policy::restrictive();
        let req = CapabilityRequest::fs_read("./src/main.rs");
        assert!(policy.check(&req).is_allowed());
    }

    #[test]
    fn test_parse_toml() {
        let toml = r#"
[allow]
fs_read = ["./", "/tmp/**"]
net_http = ["api.anthropic.com"]

[deny]
all = ["exec"]
"#;
        let policy = Policy::parse(toml).unwrap();
        
        // Allowed
        assert!(policy.check(&CapabilityRequest::fs_read("./foo.txt")).is_allowed());
        assert!(policy.check(&CapabilityRequest::fs_read("/tmp/bar/baz")).is_allowed());
        assert!(policy.check(&CapabilityRequest::net_http("api.anthropic.com")).is_allowed());
        
        // Denied
        assert!(!policy.check(&CapabilityRequest::exec("ls")).is_allowed());
        assert!(!policy.check(&CapabilityRequest::net_http("evil.com")).is_allowed());
    }
}
