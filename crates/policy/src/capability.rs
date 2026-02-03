use serde::{Deserialize, Serialize};

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
