use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use super::{SshConfig, HostEntry};

/// Simple line-by-line parser that doesn't use pest
pub fn parse_ssh_config(content: &str) -> Result<SshConfig> {
    let mut visited_files = HashSet::new();
    let mut configs_to_merge = Vec::new();
    let mut files_to_process = vec![(content.to_string(), 0)];

    while let Some((current_content, depth)) = files_to_process.pop() {
        if depth > 10 {
            anyhow::bail!("Include depth limit exceeded (max 10)");
        }

        let mut config = SshConfig {
            hosts: Vec::new(),
            global_options: HashMap::new(),
        };

        let mut current_host: Option<HostEntry> = None;
        let mut includes_to_process: Vec<PathBuf> = Vec::new();

        for line in current_content.lines() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Split on whitespace
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let key = parts[0];
            let value = if parts.len() > 1 {
                parts[1..].join(" ")
            } else {
                continue;
            };

            // Check if it's a Host entry
            if key.eq_ignore_ascii_case("Host") {
                // Save previous host if exists
                if let Some(host) = current_host.take() {
                    config.hosts.push(host);
                }
                current_host = Some(HostEntry::new(value));
                continue;
            }

            // Check if it's an Include directive
            if key.eq_ignore_ascii_case("Include") {
                let include_path = expand_tilde(&value);
                includes_to_process.push(include_path);
                continue;
            }

            // It's an option
            if let Some(ref mut host) = current_host {
                // Host-specific option
                apply_option_to_host(host, key, &value);
            } else {
                // Global option
                config.global_options.insert(key.to_string(), value);
            }
        }

        // Save last host if exists
        if let Some(host) = current_host {
            config.hosts.push(host);
        }

        configs_to_merge.push(config);

        // Process includes
        for include_path in includes_to_process {
            if visited_files.contains(&include_path) {
                continue;
            }

            if !include_path.exists() {
                continue;
            }

            visited_files.insert(include_path.clone());

            match std::fs::read_to_string(&include_path) {
                Ok(include_content) => {
                    files_to_process.push((include_content, depth + 1));
                }
                Err(_) => {}
            }
        }
    }

    // Merge all configs
    let mut final_config = SshConfig {
        hosts: Vec::new(),
        global_options: HashMap::new(),
    };

    for config in configs_to_merge {
        final_config.hosts.extend(config.hosts);
        final_config.global_options.extend(config.global_options);
    }

    Ok(final_config)
}

fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

fn apply_option_to_host(host: &mut HostEntry, key: &str, value: &str) {
    match key.to_lowercase().as_str() {
        "hostname" => host.hostname = Some(value.to_string()),
        "user" => host.user = Some(value.to_string()),
        "port" => host.port = value.parse().ok(),
        "identityfile" => host.identity_file = Some(PathBuf::from(value)),
        "proxyjump" => host.proxy_jump = Some(value.to_string()),
        "proxycommand" => host.proxy_command = Some(value.to_string()),
        "forwardagent" => host.forward_agent = parse_yes_no(value),
        "stricthostkeychecking" => host.strict_host_key_checking = Some(value.to_string()),
        "serveraliveinterval" => host.server_alive_interval = value.parse().ok(),
        "serveralivecountmax" => host.server_alive_count_max = value.parse().ok(),
        "compression" => host.compression = parse_yes_no(value),
        "connectionattempts" => host.connection_attempts = value.parse().ok(),
        "connecttimeout" => host.connect_timeout = value.parse().ok(),
        "localforward" => host.local_forward = Some(value.to_string()),
        "remoteforward" => host.remote_forward = Some(value.to_string()),
        "dynamicforward" => host.dynamic_forward = Some(value.to_string()),
        "pubkeyacceptedkeytypes" => host.pubkey_accepted_key_types = Some(value.to_string()),
        "hostkeyalgorithms" => host.host_key_algorithms = Some(value.to_string()),
        _ => {
            host.extra_options.insert(key.to_string(), value.to_string());
        }
    }
}

fn parse_yes_no(value: &str) -> Option<bool> {
    match value.to_lowercase().as_str() {
        "yes" | "true" => Some(true),
        "no" | "false" => Some(false),
        _ => None,
    }
}
