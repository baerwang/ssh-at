use ssh_at_core::config::{SshConfig, HostEntry};
use std::net::IpAddr;
use serde::Serialize;

// 定义结构化的验证错误，前端根据 code 查翻译
#[derive(Serialize)]
#[serde(tag = "code", content = "params")]
enum ValidationError {
    HostRequired,
    HostNameRequired,
    HostNameEmpty,
    HostNameInvalidDomain { value: String },
    HostNameTooLong { length: usize },
    HostNameInvalidIp { value: String },
    HostNameConsecutiveDots,
    HostNameLabelTooLong { label: String },
    HostNameLabelInvalidHyphen { label: String },
    HostNameInvalidChar { ch: String, label: String },
    PortInvalid,
    UserRequired,
    IdentityFileRequired,
    IdentityFileEmpty,
    IdentityFileExpandFailed,
    IdentityFileNotExist { path: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 序列化为 JSON，前端解析后查翻译
        write!(f, "{}", serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string()))
    }
}

impl From<ValidationError> for String {
    fn from(err: ValidationError) -> String {
        serde_json::to_string(&err).unwrap_or_else(|_| "{}".to_string())
    }
}

// Validation helper: check if hostname is valid IP or domain
fn validate_hostname(hostname: &str) -> Result<(), ValidationError> {
    let trimmed = hostname.trim();

    if trimmed.is_empty() {
        return Err(ValidationError::HostNameEmpty);
    }

    // Try parse as IP address with proper validation
    if let Ok(ip) = trimmed.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(ipv4) => {
                if ipv4.is_unspecified() {
                    return Err(ValidationError::HostNameInvalidIp { value: trimmed.to_string() });
                }
            }
            IpAddr::V6(ipv6) => {
                if ipv6.is_unspecified() {
                    return Err(ValidationError::HostNameInvalidIp { value: trimmed.to_string() });
                }
            }
        }
        return Ok(());
    }

    // Validate as domain name (RFC 1035 rules)
    if !trimmed.contains('.') {
        return Err(ValidationError::HostNameInvalidDomain { value: trimmed.to_string() });
    }

    if trimmed.len() > 253 {
        return Err(ValidationError::HostNameTooLong { length: trimmed.len() });
    }

    // Validate each label (segment between dots)
    for label in trimmed.split('.') {
        if label.is_empty() {
            return Err(ValidationError::HostNameConsecutiveDots);
        }
        if label.len() > 63 {
            return Err(ValidationError::HostNameLabelTooLong { label: label.to_string() });
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(ValidationError::HostNameLabelInvalidHyphen { label: label.to_string() });
        }
        for ch in label.chars() {
            if !ch.is_alphanumeric() && ch != '-' {
                return Err(ValidationError::HostNameInvalidChar {
                    ch: ch.to_string(),
                    label: label.to_string()
                });
            }
        }
    }

    Ok(())
}

// Validation helper: check if port is in valid range
fn validate_port(port: u16) -> Result<(), ValidationError> {
    if port == 0 {
        return Err(ValidationError::PortInvalid);
    }
    Ok(())
}

// Validation helper: check if identity file exists (REQUIRED field)
fn validate_identity_file(identity_file: &Option<std::path::PathBuf>) -> Result<(), ValidationError> {
    // Identity file is required for SSH authentication
    let identity_file = identity_file.as_ref()
        .ok_or(ValidationError::IdentityFileRequired)?;

    let path_str = identity_file.to_string_lossy();
    let trimmed = path_str.trim();

    if trimmed.is_empty() {
        return Err(ValidationError::IdentityFileEmpty);
    }

    let expanded_path = if let Some(stripped) = trimmed.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(stripped)
        } else {
            return Err(ValidationError::IdentityFileExpandFailed);
        }
    } else {
        identity_file.clone()
    };

    if !expanded_path.exists() {
        return Err(ValidationError::IdentityFileNotExist { path: trimmed.to_string() });
    }

    Ok(())
}

// Unified validation for HostEntry
fn validate_entry(entry: &HostEntry) -> Result<(), ValidationError> {
    // Validate host name
    if entry.host.trim().is_empty() {
        return Err(ValidationError::HostRequired);
    }

    // Validate hostname (IP or domain)
    if let Some(ref hostname) = entry.hostname {
        validate_hostname(hostname)?;
    } else {
        return Err(ValidationError::HostNameRequired);
    }

    // Validate user
    if entry.user.as_ref().is_none_or(|u| u.trim().is_empty()) {
        return Err(ValidationError::UserRequired);
    }

    // Validate port if provided
    if let Some(port) = entry.port {
        validate_port(port)?;
    }

    // Validate identity file (REQUIRED)
    validate_identity_file(&entry.identity_file)?;

    Ok(())
}

#[tauri::command]
pub async fn test_simple() -> Result<String, String> {
    Ok("Hello".to_string())
}

#[tauri::command]
pub async fn load_ssh_config() -> Result<SshConfig, String> {
    eprintln!("[COMMAND] load_ssh_config called");

    // Use spawn_blocking to execute in a dedicated blocking thread pool
    let config = tokio::task::spawn_blocking(|| {
        eprintln!("[COMMAND] In blocking thread pool");

        let handle = std::thread::Builder::new()
            .name("load-config-sync".to_string())
            .stack_size(64 * 1024 * 1024) // 64MB stack
            .spawn(|| {
                eprintln!("[COMMAND] In 64MB stack thread");

                let home = dirs::home_dir()
                    .ok_or("Failed to get home directory")?;
                let config_path = home.join(".ssh/config");

                if !config_path.exists() {
                    return Ok(SshConfig {
                        hosts: vec![],
                        global_options: std::collections::HashMap::new(),
                    });
                }

                let content = std::fs::read_to_string(&config_path)
                    .map_err(|e| format!("Failed to read config: {}", e))?;

                eprintln!("[COMMAND] Parsing {} bytes", content.len());

                ssh_at_core::config::simple_parser::parse_ssh_config(&content)
                    .map_err(|e| format!("Parse error: {}", e))
            })
            .map_err(|e| format!("Failed to spawn thread: {}", e))?;

        handle.join()
            .map_err(|e| format!("Thread panicked: {:?}", e))?
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    eprintln!("[COMMAND] Successfully loaded {} hosts", config.hosts.len());
    Ok(config)
}

#[tauri::command]
pub async fn save_ssh_config(config: SshConfig) -> Result<(), String> {
    ssh_at_core::config::save_config(config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_host(entry: HostEntry) -> Result<(), String> {
    validate_entry(&entry)?;

    ssh_at_core::config::add_host(entry)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_host(name: String, entry: HostEntry) -> Result<(), String> {
    validate_entry(&entry)?;

    ssh_at_core::config::update_host(&name, entry)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_host(name: String) -> Result<(), String> {
    ssh_at_core::config::delete_host(&name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_hosts(query: String) -> Result<Vec<HostEntry>, String> {
    ssh_at_core::config::search_hosts(&query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn serialize_ssh_config(config: SshConfig) -> Result<String, String> {
    ssh_at_core::config::serialize_ssh_config(&config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn parse_ssh_config(content: String) -> Result<SshConfig, String> {
    ssh_at_core::config::simple_parser::parse_ssh_config(&content)
        .map_err(|e| e.to_string())
}
