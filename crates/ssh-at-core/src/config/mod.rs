pub mod parser;
pub mod simple_parser;
pub mod crud;

use serde::{Deserialize, Serialize, Deserializer};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub hosts: Vec<HostEntry>,
    pub global_options: HashMap<String, String>,
}

// Simplified version for Tauri IPC - avoid nested HashMap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostEntrySimple {
    pub host: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,  // PathBuf -> String
    pub proxy_jump: Option<String>,
    pub proxy_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfigSimple {
    pub hosts: Vec<HostEntrySimple>,
}

// Custom deserializer for PathBuf that accepts both String and PathBuf
fn deserialize_pathbuf<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;

    match value {
        None => Ok(None),
        Some(serde_json::Value::String(s)) => {
            if s.is_empty() {
                Ok(None)
            } else {
                Ok(Some(PathBuf::from(s)))
            }
        },
        Some(v) => Err(D::Error::custom(format!("Expected string for identity_file, got: {:?}", v))),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostEntry {
    pub host: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    #[serde(deserialize_with = "deserialize_pathbuf")]
    pub identity_file: Option<PathBuf>,
    pub proxy_jump: Option<String>,
    pub proxy_command: Option<String>,
    pub forward_agent: Option<bool>,
    pub strict_host_key_checking: Option<String>,
    pub server_alive_interval: Option<u32>,
    pub server_alive_count_max: Option<u32>,
    pub compression: Option<bool>,
    pub connection_attempts: Option<u32>,
    pub connect_timeout: Option<u32>,
    pub local_forward: Option<String>,
    pub remote_forward: Option<String>,
    pub dynamic_forward: Option<String>,
    pub pubkey_accepted_key_types: Option<String>,
    pub host_key_algorithms: Option<String>,
    pub extra_options: HashMap<String, String>,
}

impl HostEntry {
    pub fn new(host: String) -> Self {
        Self {
            host,
            hostname: None,
            user: None,
            port: None,
            identity_file: None,
            proxy_jump: None,
            proxy_command: None,
            forward_agent: None,
            strict_host_key_checking: None,
            server_alive_interval: None,
            server_alive_count_max: None,
            compression: None,
            connection_attempts: None,
            connect_timeout: None,
            local_forward: None,
            remote_forward: None,
            dynamic_forward: None,
            pubkey_accepted_key_types: None,
            host_key_algorithms: None,
            extra_options: HashMap::new(),
        }
    }
}

// Re-export CRUD functions
pub use crud::{load_config, load_config_sync, save_config, add_host, update_host, delete_host, search_hosts, serialize_ssh_config};
