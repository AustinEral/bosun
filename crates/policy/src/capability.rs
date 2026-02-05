use serde::{Deserialize, Serialize};
use std::fmt;

/// Capability types that can be granted or denied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    FsRead,
    FsWrite,
    NetHttp,
    Exec,
    SecretsRead,
}

impl CapabilityKind {
    /// Returns the canonical name of this capability kind.
    ///
    /// Names match the serde serialization format (snake_case).
    ///
    /// # Example
    ///
    /// ```
    /// use policy::CapabilityKind;
    ///
    /// assert_eq!(CapabilityKind::FsRead.name(), "fs_read");
    /// assert_eq!(CapabilityKind::NetHttp.name(), "net_http");
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            Self::FsRead => "fs_read",
            Self::FsWrite => "fs_write",
            Self::NetHttp => "net_http",
            Self::Exec => "exec",
            Self::SecretsRead => "secrets_read",
        }
    }
}

impl fmt::Display for CapabilityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A capability request with optional scope.
#[derive(Debug, Clone)]
pub struct CapabilityRequest {
    pub kind: CapabilityKind,
    pub scope: Option<String>, // e.g., path, domain, command
}

impl CapabilityRequest {
    pub fn new(kind: CapabilityKind) -> Self {
        Self { kind, scope: None }
    }

    pub fn with_scope(kind: CapabilityKind, scope: impl Into<String>) -> Self {
        Self {
            kind,
            scope: Some(scope.into()),
        }
    }

    pub fn fs_read(path: impl Into<String>) -> Self {
        Self::with_scope(CapabilityKind::FsRead, path)
    }

    pub fn fs_write(path: impl Into<String>) -> Self {
        Self::with_scope(CapabilityKind::FsWrite, path)
    }

    pub fn net_http(domain: impl Into<String>) -> Self {
        Self::with_scope(CapabilityKind::NetHttp, domain)
    }

    pub fn exec(command: impl Into<String>) -> Self {
        Self::with_scope(CapabilityKind::Exec, command)
    }

    pub fn secrets_read(key: impl Into<String>) -> Self {
        Self::with_scope(CapabilityKind::SecretsRead, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_kind_name_matches_serde() {
        // Verify that name() returns the same string as serde serialization
        let kinds = [
            (CapabilityKind::FsRead, "fs_read"),
            (CapabilityKind::FsWrite, "fs_write"),
            (CapabilityKind::NetHttp, "net_http"),
            (CapabilityKind::Exec, "exec"),
            (CapabilityKind::SecretsRead, "secrets_read"),
        ];

        for (kind, expected) in kinds {
            assert_eq!(kind.name(), expected);
            // Also verify serde produces the same result
            let serialized = serde_json::to_string(&kind).unwrap();
            assert_eq!(serialized, format!("\"{}\"", expected));
        }
    }

    #[test]
    fn capability_kind_display_uses_name() {
        // Verify Display produces the same output as name()
        for kind in [
            CapabilityKind::FsRead,
            CapabilityKind::FsWrite,
            CapabilityKind::NetHttp,
            CapabilityKind::Exec,
            CapabilityKind::SecretsRead,
        ] {
            assert_eq!(kind.to_string(), kind.name());
        }
    }
}
