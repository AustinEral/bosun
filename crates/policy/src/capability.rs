/// Capabilities that can be granted or denied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capability {
    FsRead { paths: Vec<String> },
    FsWrite { paths: Vec<String> },
    NetHttp { domains: Vec<String> },
    Exec { commands: Vec<String> },
    SecretsRead { keys: Vec<String> },
}
