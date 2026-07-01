use super::{SshConfig, HostEntry};
use super::simple_parser::parse_ssh_config;
use anyhow::Result;
use std::path::PathBuf;

#[cfg(test)]
use tokio::fs;

#[cfg(test)]
use ssh_at_storage::backup::create_backup;

/// Get the SSH config file path
fn get_ssh_config_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".ssh").join("config"))
        .unwrap_or_else(|| PathBuf::from(".ssh/config"))
}

#[cfg(test)]
fn get_ssh_config_path_with_home(home_override: Option<PathBuf>) -> PathBuf {
    if let Some(h) = home_override {
        h.join(".ssh").join("config")
    } else {
        dirs::home_dir()
            .map(|h| h.join(".ssh").join("config"))
            .unwrap_or_else(|| PathBuf::from(".ssh/config"))
    }
}

/// Load SSH config from path (for testing)
#[cfg(test)]
async fn load_config_from_path(config_path: &PathBuf) -> Result<SshConfig> {
    if !config_path.exists() {
        return Ok(SshConfig {
            hosts: vec![],
            global_options: std::collections::HashMap::new(),
        });
    }

    let content = fs::read_to_string(config_path).await?;
    parse_ssh_config(&content)
}

/// Save SSH config to path (for testing, no backup)
#[cfg(test)]
async fn save_config_to_path(config: SshConfig, config_path: &PathBuf) -> Result<()> {
    // Ensure .ssh directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let content = serialize_ssh_config(&config)?;

    // Atomic write: write to temp file, then rename
    let temp_path = config_path.with_extension("tmp");
    fs::write(&temp_path, content).await?;
    fs::rename(&temp_path, config_path).await?;

    Ok(())
}

/// Load SSH config from ~/.ssh/config (synchronous version)
pub fn load_config_sync() -> Result<SshConfig> {
    let config_path = get_ssh_config_path();

    if !config_path.exists() {
        return Ok(SshConfig {
            hosts: vec![],
            global_options: std::collections::HashMap::new(),
        });
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| anyhow::anyhow!("Failed to read config: {}", e))?;

    let result = parse_ssh_config(&content)?;

    Ok(result)
}

/// Load SSH config from ~/.ssh/config
pub async fn load_config() -> Result<SshConfig> {
    let config_path = get_ssh_config_path();

    if !config_path.exists() {
        return Ok(SshConfig {
            hosts: vec![],
            global_options: std::collections::HashMap::new(),
        });
    }

    // Use std::sync::mpsc instead of tokio::sync::oneshot to avoid runtime dependency
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024) // 32MB stack
        .spawn(move || {
            eprintln!("[CRUD] Custom thread (32MB stack): reading config file");
            let result = (|| -> Result<SshConfig> {
                let content = std::fs::read_to_string(&config_path)
                    .map_err(|e| anyhow::anyhow!("Failed to read config: {}", e))?;
                eprintln!("[CRUD] Custom thread: parsing {} bytes", content.len());
                let result = parse_ssh_config(&content)?;
                eprintln!("[CRUD] Custom thread: parsed {} hosts", result.hosts.len());
                Ok(result)
            })();
            eprintln!("[CRUD] Custom thread: sending result back");
            let _ = tx.send(result);
        })
        .map_err(|e| anyhow::anyhow!("Failed to spawn thread: {}", e))?;

    // Use tokio::task::spawn_blocking to wait for std::sync::mpsc in async context
    tokio::task::spawn_blocking(move || {
        rx.recv()
            .map_err(|e| anyhow::anyhow!("Thread panicked or channel closed: {}", e))?
    })
    .await
    .map_err(|e| anyhow::anyhow!("Join error: {}", e))?
}

/// Save SSH config to ~/.ssh/config
pub async fn save_config(config: SshConfig) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let config_path = get_ssh_config_path();

        // Create backup before saving
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                ssh_at_storage::backup::create_backup(&content).await
            })?;
        }

        // Ensure .ssh directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serialize_ssh_config(&config)?;

        // Atomic write: write to temp file, then rename
        let temp_path = config_path.with_extension("tmp");
        std::fs::write(&temp_path, content)?;
        std::fs::rename(&temp_path, &config_path)?;

        Ok(())
    })
    .await
    .map_err(|e| anyhow::anyhow!("Join error: {}", e))?
}

/// Add a new host entry
pub async fn add_host(entry: HostEntry) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::Builder::new()
        .name("add-host-64mb".to_string())
        .stack_size(64 * 1024 * 1024) // 64MB stack to avoid overflow
        .spawn(move || {
            let result = (|| -> Result<()> {
                let mut config = load_config_sync()?;
                config.hosts.push(entry);

                let config_path = get_ssh_config_path();
                if config_path.exists() {
                    let content = std::fs::read_to_string(&config_path)?;
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(async {
                        ssh_at_storage::backup::create_backup(&content).await
                    })?;
                }

                if let Some(parent) = config_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let content = serialize_ssh_config(&config)?;
                let temp_path = config_path.with_extension("tmp");
                std::fs::write(&temp_path, content)?;
                std::fs::rename(&temp_path, &config_path)?;

                Ok(())
            })();
            let _ = tx.send(result);
        })
        .map_err(|e| anyhow::anyhow!("Failed to spawn thread: {}", e))?;

    tokio::task::spawn_blocking(move || {
        rx.recv()
            .map_err(|e| anyhow::anyhow!("Thread panicked or channel closed: {}", e))?
    })
    .await
    .map_err(|e| anyhow::anyhow!("Join error: {}", e))?
}

/// Update an existing host entry
pub async fn update_host(name: &str, entry: HostEntry) -> Result<()> {
    let name = name.to_string();
    let entry_clone = entry.clone();

    tokio::task::spawn_blocking(move || {
        let handle = std::thread::Builder::new()
            .name("update-host-worker".to_string())
            .stack_size(64 * 1024 * 1024) // 64MB stack, same as load_ssh_config
            .spawn(move || -> Result<()> {
                let mut config = load_config_sync()?;

                if let Some(pos) = config.hosts.iter().position(|h| h.host == name) {
                    config.hosts[pos] = entry_clone;

                    let config_path = get_ssh_config_path();

                    // Create backup before modifying
                    if config_path.exists() {
                        let content = std::fs::read_to_string(&config_path)?;
                        // Create runtime for async backup call
                        let rt = tokio::runtime::Runtime::new()?;
                        rt.block_on(async {
                            ssh_at_storage::backup::create_backup(&content).await
                        })?;
                    }

                    if let Some(parent) = config_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    let content = serialize_ssh_config(&config)?;
                    let temp_path = config_path.with_extension("tmp");
                    std::fs::write(&temp_path, content)?;
                    std::fs::rename(&temp_path, &config_path)?;

                    Ok(())
                } else {
                    anyhow::bail!("Host '{}' not found", name)
                }
            })
            .map_err(|e| anyhow::anyhow!("Failed to spawn thread: {}", e))?;

        handle.join()
            .map_err(|e| anyhow::anyhow!("Thread panicked: {:?}", e))?
    })
    .await
    .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
}

/// Delete a host entry
pub async fn delete_host(name: &str) -> Result<()> {
    let name = name.to_string();
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::Builder::new()
        .name("delete-host-64mb".to_string())
        .stack_size(64 * 1024 * 1024) // 64MB stack to avoid overflow
        .spawn(move || {
            let result = (|| -> Result<()> {
                let mut config = load_config_sync()?;
                config.hosts.retain(|h| h.host != name);

                let config_path = get_ssh_config_path();
                if config_path.exists() {
                    let content = std::fs::read_to_string(&config_path)?;
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(async {
                        ssh_at_storage::backup::create_backup(&content).await
                    })?;
                }

                if let Some(parent) = config_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let content = serialize_ssh_config(&config)?;
                let temp_path = config_path.with_extension("tmp");
                std::fs::write(&temp_path, content)?;
                std::fs::rename(&temp_path, &config_path)?;

                Ok(())
            })();
            let _ = tx.send(result);
        })
        .map_err(|e| anyhow::anyhow!("Failed to spawn thread: {}", e))?;

    tokio::task::spawn_blocking(move || {
        rx.recv()
            .map_err(|e| anyhow::anyhow!("Thread panicked or channel closed: {}", e))?
    })
    .await
    .map_err(|e| anyhow::anyhow!("Join error: {}", e))?
}

/// Search hosts by query (matches host, hostname, or user)
pub async fn search_hosts(query: &str) -> Result<Vec<HostEntry>> {
    let query = query.to_string();
    tokio::task::spawn_blocking(move || {
        let config = load_config_sync()?;
        let query_lower = query.to_lowercase();

        let results = config.hosts.into_iter()
            .filter(|h| {
                h.host.to_lowercase().contains(&query_lower) ||
                h.hostname.as_ref().is_some_and(|hn| hn.to_lowercase().contains(&query_lower)) ||
                h.user.as_ref().is_some_and(|u| u.to_lowercase().contains(&query_lower))
            })
            .collect();

        Ok(results)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Join error: {}", e))?
}

/// Serialize SSH config to string
pub fn serialize_ssh_config(config: &SshConfig) -> Result<String> {
    let mut output = String::new();

    // Write global options first
    for (key, value) in &config.global_options {
        output.push_str(&format!("{} {}\n", key, value));
    }
    if !config.global_options.is_empty() {
        output.push('\n');
    }

    // Write host entries
    for host in &config.hosts {
        output.push_str(&format!("Host {}\n", host.host));

        if let Some(ref hostname) = host.hostname {
            output.push_str(&format!("  HostName {}\n", hostname));
        }
        if let Some(ref user) = host.user {
            output.push_str(&format!("  User {}\n", user));
        }
        if let Some(port) = host.port {
            output.push_str(&format!("  Port {}\n", port));
        }
        if let Some(ref identity_file) = host.identity_file {
            output.push_str(&format!("  IdentityFile {}\n", identity_file.display()));
        }
        if let Some(ref proxy_jump) = host.proxy_jump {
            output.push_str(&format!("  ProxyJump {}\n", proxy_jump));
        }
        if let Some(ref proxy_command) = host.proxy_command {
            output.push_str(&format!("  ProxyCommand {}\n", proxy_command));
        }
        if let Some(forward_agent) = host.forward_agent {
            output.push_str(&format!("  ForwardAgent {}\n", if forward_agent { "yes" } else { "no" }));
        }
        if let Some(ref strict) = host.strict_host_key_checking {
            output.push_str(&format!("  StrictHostKeyChecking {}\n", strict));
        }
        if let Some(interval) = host.server_alive_interval {
            output.push_str(&format!("  ServerAliveInterval {}\n", interval));
        }
        if let Some(count) = host.server_alive_count_max {
            output.push_str(&format!("  ServerAliveCountMax {}\n", count));
        }
        if let Some(compression) = host.compression {
            output.push_str(&format!("  Compression {}\n", if compression { "yes" } else { "no" }));
        }
        if let Some(attempts) = host.connection_attempts {
            output.push_str(&format!("  ConnectionAttempts {}\n", attempts));
        }
        if let Some(timeout) = host.connect_timeout {
            output.push_str(&format!("  ConnectTimeout {}\n", timeout));
        }
        if let Some(ref local_fwd) = host.local_forward {
            output.push_str(&format!("  LocalForward {}\n", local_fwd));
        }
        if let Some(ref remote_fwd) = host.remote_forward {
            output.push_str(&format!("  RemoteForward {}\n", remote_fwd));
        }
        if let Some(ref dynamic_fwd) = host.dynamic_forward {
            output.push_str(&format!("  DynamicForward {}\n", dynamic_fwd));
        }
        if let Some(ref pubkey_types) = host.pubkey_accepted_key_types {
            output.push_str(&format!("  PubkeyAcceptedKeyTypes {}\n", pubkey_types));
        }
        if let Some(ref host_key_algos) = host.host_key_algorithms {
            output.push_str(&format!("  HostKeyAlgorithms {}\n", host_key_algos));
        }

        // Extra options
        for (key, value) in &host.extra_options {
            output.push_str(&format!("  {} {}\n", key, value));
        }

        output.push('\n');
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_host(name: &str) -> HostEntry {
        HostEntry {
            host: name.to_string(),
            hostname: Some("example.com".to_string()),
            user: Some("testuser".to_string()),
            port: Some(22),
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

    #[tokio::test]
    async fn test_serialize_and_parse_roundtrip() {
        let mut config = SshConfig {
            hosts: vec![
                create_test_host("server1"),
                create_test_host("server2"),
            ],
            global_options: HashMap::new(),
        };
        config.global_options.insert("AddKeysToAgent".to_string(), "yes".to_string());

        let serialized = serialize_ssh_config(&config).unwrap();
        let parsed = parse_ssh_config(&serialized).unwrap();

        assert_eq!(parsed.hosts.len(), 2);
        assert_eq!(parsed.hosts[0].host, "server1");
        assert_eq!(parsed.hosts[1].host, "server2");
        assert_eq!(parsed.global_options.get("AddKeysToAgent"), Some(&"yes".to_string()));
    }

    #[tokio::test]
    async fn test_serialize_all_fields() {
        let host = HostEntry {
            host: "full".to_string(),
            hostname: Some("full.com".to_string()),
            user: Some("admin".to_string()),
            port: Some(2222),
            identity_file: Some(PathBuf::from("~/.ssh/id_rsa")),
            proxy_jump: Some("bastion".to_string()),
            proxy_command: Some("ssh -W %h:%p bastion".to_string()),
            forward_agent: Some(true),
            strict_host_key_checking: Some("yes".to_string()),
            server_alive_interval: Some(60),
            server_alive_count_max: Some(3),
            compression: Some(false),
            connection_attempts: Some(5),
            connect_timeout: Some(30),
            local_forward: Some("8080 localhost:80".to_string()),
            remote_forward: Some("9090 localhost:90".to_string()),
            dynamic_forward: Some("1080".to_string()),
            pubkey_accepted_key_types: None,
            host_key_algorithms: None,
            extra_options: {
                let mut map = HashMap::new();
                map.insert("CustomOption".to_string(), "value".to_string());
                map
            },
        };

        let config = SshConfig {
            hosts: vec![host],
            global_options: HashMap::new(),
        };

        let serialized = serialize_ssh_config(&config).unwrap();

        assert!(serialized.contains("Host full"));
        assert!(serialized.contains("HostName full.com"));
        assert!(serialized.contains("User admin"));
        assert!(serialized.contains("Port 2222"));
        assert!(serialized.contains("IdentityFile"));
        assert!(serialized.contains("ProxyJump bastion"));
        assert!(serialized.contains("ProxyCommand"));
        assert!(serialized.contains("ForwardAgent yes"));
        assert!(serialized.contains("StrictHostKeyChecking yes"));
        assert!(serialized.contains("ServerAliveInterval 60"));
        assert!(serialized.contains("ServerAliveCountMax 3"));
        assert!(serialized.contains("Compression no"));
        assert!(serialized.contains("ConnectionAttempts 5"));
        assert!(serialized.contains("ConnectTimeout 30"));
        assert!(serialized.contains("LocalForward 8080"));
        assert!(serialized.contains("RemoteForward 9090"));
        assert!(serialized.contains("DynamicForward 1080"));
        assert!(serialized.contains("CustomOption value"));
    }

    #[tokio::test]
    async fn test_add_host() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".ssh").join("config");

        let host = create_test_host("testserver");
        let mut config = SshConfig {
            hosts: vec![],
            global_options: HashMap::new(),
        };
        config.hosts.push(host);

        save_config_to_path(config, &config_path).await.unwrap();
        let loaded = load_config_from_path(&config_path).await.unwrap();

        assert_eq!(loaded.hosts.len(), 1);
        assert_eq!(loaded.hosts[0].host, "testserver");
        assert_eq!(loaded.hosts[0].hostname, Some("example.com".to_string()));
    }

    #[tokio::test]
    async fn test_update_host() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".ssh").join("config");

        let mut config = SshConfig {
            hosts: vec![create_test_host("testserver")],
            global_options: HashMap::new(),
        };
        save_config_to_path(config.clone(), &config_path).await.unwrap();

        // Update
        config.hosts[0].hostname = Some("updated.com".to_string());
        config.hosts[0].port = Some(2222);
        save_config_to_path(config, &config_path).await.unwrap();

        let loaded = load_config_from_path(&config_path).await.unwrap();
        assert_eq!(loaded.hosts.len(), 1);
        assert_eq!(loaded.hosts[0].hostname, Some("updated.com".to_string()));
        assert_eq!(loaded.hosts[0].port, Some(2222));
    }

    #[tokio::test]
    async fn test_delete_host() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".ssh").join("config");

        let mut config = SshConfig {
            hosts: vec![
                create_test_host("server1"),
                create_test_host("server2"),
            ],
            global_options: HashMap::new(),
        };
        save_config_to_path(config.clone(), &config_path).await.unwrap();

        // Delete server1
        config.hosts.retain(|h| h.host != "server1");
        save_config_to_path(config, &config_path).await.unwrap();

        let loaded = load_config_from_path(&config_path).await.unwrap();
        assert_eq!(loaded.hosts.len(), 1);
        assert_eq!(loaded.hosts[0].host, "server2");
    }

    #[tokio::test]
    async fn test_search_hosts() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".ssh").join("config");

        let config = SshConfig {
            hosts: vec![
                create_test_host("prod-server"),
                create_test_host("dev-server"),
                create_test_host("staging-db"),
            ],
            global_options: HashMap::new(),
        };
        save_config_to_path(config.clone(), &config_path).await.unwrap();

        // Search by host name
        let query_lower = "server".to_lowercase();
        let results: Vec<_> = config.hosts.iter()
            .filter(|h| h.host.to_lowercase().contains(&query_lower))
            .collect();
        assert_eq!(results.len(), 2);

        // Search by specific prefix
        let query_lower = "prod".to_lowercase();
        let results: Vec<_> = config.hosts.iter()
            .filter(|h| h.host.to_lowercase().contains(&query_lower))
            .collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].host, "prod-server");
    }

    #[tokio::test]
    async fn test_search_hosts_by_hostname() {
        let mut host1 = create_test_host("server1");
        host1.hostname = Some("prod.example.com".to_string());

        let mut host2 = create_test_host("server2");
        host2.hostname = Some("dev.example.com".to_string());

        let config = SshConfig {
            hosts: vec![host1, host2],
            global_options: HashMap::new(),
        };

        let query_lower = "prod".to_lowercase();
        let results: Vec<_> = config.hosts.iter()
            .filter(|h| h.hostname.as_ref().map_or(false, |hn| hn.to_lowercase().contains(&query_lower)))
            .collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].host, "server1");
    }

    #[tokio::test]
    async fn test_search_hosts_by_user() {
        let mut host1 = create_test_host("server1");
        host1.user = Some("admin".to_string());

        let mut host2 = create_test_host("server2");
        host2.user = Some("deploy".to_string());

        let config = SshConfig {
            hosts: vec![host1, host2],
            global_options: HashMap::new(),
        };

        let query_lower = "admin".to_lowercase();
        let results: Vec<_> = config.hosts.iter()
            .filter(|h| h.user.as_ref().map_or(false, |u| u.to_lowercase().contains(&query_lower)))
            .collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].host, "server1");
    }

    #[tokio::test]
    async fn test_search_case_insensitive() {
        let config = SshConfig {
            hosts: vec![create_test_host("PROD-Server")],
            global_options: HashMap::new(),
        };

        let query_lower = "prod".to_lowercase();
        let results: Vec<_> = config.hosts.iter()
            .filter(|h| h.host.to_lowercase().contains(&query_lower))
            .collect();
        assert_eq!(results.len(), 1);

        let query_lower = "PROD".to_lowercase();
        let results: Vec<_> = config.hosts.iter()
            .filter(|h| h.host.to_lowercase().contains(&query_lower))
            .collect();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_load_empty_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".ssh").join("config");

        let config = load_config_from_path(&config_path).await.unwrap();
        assert_eq!(config.hosts.len(), 0);
        assert_eq!(config.global_options.len(), 0);
    }

    #[tokio::test]
    async fn test_backup_creation() {
        let config_content = "Host test\n  HostName test.com\n";

        let backup_info = create_backup(config_content).await.unwrap();
        assert!(!backup_info.file_path.is_empty());
        assert!(backup_info.file_path.contains("backup"));
        assert_eq!(backup_info.host_count, 1);
    }

    #[tokio::test]
    async fn test_save_config_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".ssh").join("config");

        let config = SshConfig {
            hosts: vec![create_test_host("atomic")],
            global_options: HashMap::new(),
        };

        save_config_to_path(config, &config_path).await.unwrap();

        assert!(config_path.exists());
        let temp_path = config_path.with_extension("tmp");
        assert!(!temp_path.exists());
    }

    #[tokio::test]
    async fn test_serialize_with_global_options() {
        let mut config = SshConfig {
            hosts: vec![create_test_host("server")],
            global_options: HashMap::new(),
        };
        config.global_options.insert("AddKeysToAgent".to_string(), "yes".to_string());
        config.global_options.insert("ForwardAgent".to_string(), "no".to_string());

        let serialized = serialize_ssh_config(&config).unwrap();
        assert!(serialized.contains("AddKeysToAgent yes"));
        assert!(serialized.contains("ForwardAgent no"));
        assert!(serialized.contains("Host server"));
    }

    #[tokio::test]
    async fn test_serialize_empty_config() {
        let config = SshConfig {
            hosts: vec![],
            global_options: HashMap::new(),
        };

        let serialized = serialize_ssh_config(&config).unwrap();
        assert_eq!(serialized.trim(), "");
    }

    #[tokio::test]
    async fn test_update_host_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".ssh").join("config");

        let config = SshConfig {
            hosts: vec![create_test_host("existing")],
            global_options: HashMap::new(),
        };
        save_config_to_path(config, &config_path).await.unwrap();

        std::env::set_var("HOME", temp_dir.path());

        let result = update_host("nonexistent", create_test_host("new")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_get_ssh_config_path_with_home_override() {
        let temp_dir = TempDir::new().unwrap();
        let custom_home = temp_dir.path().to_path_buf();

        let config_path = get_ssh_config_path_with_home(Some(custom_home.clone()));
        assert_eq!(config_path, custom_home.join(".ssh").join("config"));
    }

    #[tokio::test]
    async fn test_get_ssh_config_path_without_override() {
        let config_path = get_ssh_config_path_with_home(None);
        assert!(config_path.ends_with(".ssh/config"));
    }

    #[tokio::test]
    async fn test_load_config_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("HOME", temp_dir.path());

        let config = load_config().await.unwrap();
        assert_eq!(config.hosts.len(), 0);
        assert_eq!(config.global_options.len(), 0);
    }

    #[tokio::test]
    async fn test_save_config_creates_parent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".ssh").join("config");

        let config = SshConfig {
            hosts: vec![create_test_host("initial")],
            global_options: HashMap::new(),
        };
        save_config_to_path(config, &config_path).await.unwrap();

        assert!(config_path.exists());
        let loaded = load_config_from_path(&config_path).await.unwrap();
        assert_eq!(loaded.hosts[0].host, "initial");
    }

    #[tokio::test]
    async fn test_save_config_with_backup_creates_backup() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".ssh").join("config");

        let config = SshConfig {
            hosts: vec![create_test_host("initial")],
            global_options: HashMap::new(),
        };
        save_config_to_path(config, &config_path).await.unwrap();

        let config2 = SshConfig {
            hosts: vec![create_test_host("updated")],
            global_options: HashMap::new(),
        };
        save_config_to_path(config2, &config_path).await.unwrap();

        let loaded = load_config_from_path(&config_path).await.unwrap();
        assert_eq!(loaded.hosts[0].host, "updated");
    }

    #[tokio::test]
    async fn test_add_delete_search_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".ssh").join("config");

        let mut config = SshConfig {
            hosts: vec![],
            global_options: HashMap::new(),
        };

        config.hosts.push(create_test_host("host1"));
        config.hosts.push(create_test_host("host2"));
        save_config_to_path(config, &config_path).await.unwrap();

        let mut loaded = load_config_from_path(&config_path).await.unwrap();
        assert_eq!(loaded.hosts.len(), 2);

        loaded.hosts.retain(|h| h.host != "host1");
        save_config_to_path(loaded, &config_path).await.unwrap();

        let final_config = load_config_from_path(&config_path).await.unwrap();
        assert_eq!(final_config.hosts.len(), 1);
        assert_eq!(final_config.hosts[0].host, "host2");
    }

    #[tokio::test]
    async fn test_load_and_save_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("HOME", temp_dir.path());
        let config_path = temp_dir.path().join(".ssh").join("config");

        let original = SshConfig {
            hosts: vec![
                create_test_host("server1"),
                create_test_host("server2"),
            ],
            global_options: {
                let mut map = HashMap::new();
                map.insert("Global".to_string(), "option".to_string());
                map
            },
        };

        save_config_to_path(original.clone(), &config_path).await.unwrap();
        let loaded = load_config_from_path(&config_path).await.unwrap();

        assert_eq!(loaded.hosts.len(), 2);
        assert_eq!(loaded.global_options.len(), 1);
    }
}
