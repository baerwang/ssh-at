use pest::Parser;
use pest_derive::Parser;
use anyhow::{Result, Context};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use super::{SshConfig, HostEntry};

#[derive(Parser)]
#[grammar = "config/ssh.pest"]
struct SshConfigParser;

/// Parse SSH config file content with Include support
pub fn parse_ssh_config(content: &str) -> Result<SshConfig> {
    eprintln!("[PARSER] Starting parse, content size: {} bytes", content.len());

    let mut visited_files = HashSet::new();
    let mut configs_to_merge = Vec::new();
    let mut files_to_process = vec![(content.to_string(), 0)]; // (content, depth)

    // Iterative processing instead of recursion
    while let Some((current_content, depth)) = files_to_process.pop() {
        eprintln!("[PARSER] Processing depth {}: {} bytes", depth, current_content.len());

        if depth > 10 {
            anyhow::bail!("Include depth limit exceeded (max 10)");
        }

        let pairs = SshConfigParser::parse(Rule::config, &current_content)
            .context("Failed to parse SSH config")?;

        let mut config = SshConfig {
            hosts: Vec::new(),
            global_options: HashMap::new(),
        };

        let mut current_host: Option<HostEntry> = None;
        let mut includes_to_process: Vec<PathBuf> = Vec::new();

        for pair in pairs {
            if pair.as_rule() == Rule::config {
                for line in pair.into_inner() {
                    match line.as_rule() {
                        Rule::line => {
                            if let Some(inner) = line.into_inner().next() {
                                match inner.as_rule() {
                                    Rule::host_entry => {
                                        if let Some(host) = current_host.take() {
                                            config.hosts.push(host);
                                        }
                                        let mut inner_pairs = inner.into_inner();
                                        if let Some(pattern) = inner_pairs.next() {
                                            let host_name = pattern.as_str().trim().to_string();
                                            current_host = Some(HostEntry::new(host_name));
                                        }
                                    }
                                    Rule::global_option => {
                                        let mut inner_pairs = inner.into_inner();
                                        let key = inner_pairs.next().map(|p| p.as_str().to_string());
                                        let value = inner_pairs.next().map(|p| p.as_str().trim().to_string());

                                        if let (Some(key), Some(value)) = (key, value) {
                                            if key.to_lowercase() == "include" {
                                                let include_path = expand_tilde(&value);
                                                includes_to_process.push(include_path);
                                            } else if let Some(ref mut host) = current_host {
                                                apply_option_to_host(host, &key, &value);
                                            } else {
                                                config.global_options.insert(key, value);
                                            }
                                        }
                                    }
                                    Rule::comment_line => {}
                                    _ => {}
                                }
                            }
                        }
                        Rule::EOI => {}
                        _ => {}
                    }
                }
            }
        }

        if let Some(host) = current_host {
            config.hosts.push(host);
        }

        configs_to_merge.push(config);

        // Queue includes for processing
        for include_path in includes_to_process {
            eprintln!("[PARSER] Found Include: {:?}", include_path);

            if visited_files.contains(&include_path) {
                eprintln!("[PARSER] Skipping already visited: {:?}", include_path);
                continue;
            }

            if !include_path.exists() {
                eprintln!("[PARSER] Include file not found: {:?}", include_path);
                continue;
            }

            visited_files.insert(include_path.clone());

            match std::fs::read_to_string(&include_path) {
                Ok(include_content) => {
                    eprintln!("[PARSER] Queuing Include file: {} bytes", include_content.len());
                    files_to_process.push((include_content, depth + 1));
                }
                Err(e) => {
                    eprintln!("[PARSER] Failed to read Include: {:?}", e);
                }
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

    eprintln!("[PARSER] Parse completed: {} hosts", final_config.hosts.len());
    Ok(final_config)
}

/// Expand tilde (~) in file paths
fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

/// Apply parsed option to host entry
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
        _ => {
            // Store unknown options in extra_options
            host.extra_options.insert(key.to_string(), value.to_string());
        }
    }
}

/// Parse yes/no boolean values
fn parse_yes_no(value: &str) -> Option<bool> {
    match value.to_lowercase().as_str() {
        "yes" | "true" => Some(true),
        "no" | "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_config() {
        let config_text = r#"# Global options
User defaultuser

Host example
    HostName example.com
    Port 2222
    User testuser
"#;

        let result = parse_ssh_config(config_text);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.global_options.get("User"), Some(&"defaultuser".to_string()));
        assert_eq!(config.hosts.len(), 1);
        assert_eq!(config.hosts[0].host, "example");
        assert_eq!(config.hosts[0].hostname, Some("example.com".to_string()));
        assert_eq!(config.hosts[0].port, Some(2222));
    }

    #[test]
    fn test_parse_multiple_hosts() {
        let config_text = r#"
Host server1
    HostName 192.168.1.1
    User admin

Host server2
    HostName 192.168.1.2
    User root
"#;

        let config = parse_ssh_config(config_text).unwrap();
        assert_eq!(config.hosts.len(), 2);
        assert_eq!(config.hosts[0].host, "server1");
        assert_eq!(config.hosts[1].host, "server2");
    }

    #[test]
    fn test_parse_identity_file() {
        let config_text = r#"
Host github
    HostName github.com
    IdentityFile ~/.ssh/id_rsa
"#;

        let config = parse_ssh_config(config_text).unwrap();
        assert_eq!(config.hosts[0].identity_file, Some(PathBuf::from("~/.ssh/id_rsa")));
    }

    #[test]
    fn test_parse_proxy_options() {
        let config_text = r#"
Host target
    HostName target.com
    ProxyJump bastion
    ProxyCommand ssh -W %h:%p bastion
"#;

        let config = parse_ssh_config(config_text).unwrap();
        assert_eq!(config.hosts[0].proxy_jump, Some("bastion".to_string()));
        assert_eq!(config.hosts[0].proxy_command, Some("ssh -W %h:%p bastion".to_string()));
    }

    #[test]
    fn test_parse_boolean_options() {
        let config_text = r#"
Host secure
    HostName secure.com
    ForwardAgent yes
    Compression no
"#;

        let config = parse_ssh_config(config_text).unwrap();
        assert_eq!(config.hosts[0].forward_agent, Some(true));
        assert_eq!(config.hosts[0].compression, Some(false));
    }

    #[test]
    fn test_parse_server_alive_options() {
        let config_text = r#"
Host keepalive
    HostName keepalive.com
    ServerAliveInterval 60
    ServerAliveCountMax 3
"#;

        let config = parse_ssh_config(config_text).unwrap();
        assert_eq!(config.hosts[0].server_alive_interval, Some(60));
        assert_eq!(config.hosts[0].server_alive_count_max, Some(3));
    }

    #[test]
    fn test_parse_connection_options() {
        let config_text = r#"
Host retry
    HostName retry.com
    ConnectionAttempts 5
    ConnectTimeout 30
"#;

        let config = parse_ssh_config(config_text).unwrap();
        assert_eq!(config.hosts[0].connection_attempts, Some(5));
        assert_eq!(config.hosts[0].connect_timeout, Some(30));
    }

    #[test]
    fn test_parse_forward_options() {
        let config_text = r#"
Host tunnel
    HostName tunnel.com
    LocalForward 8080 localhost:80
    RemoteForward 9090 localhost:90
    DynamicForward 1080
"#;

        let config = parse_ssh_config(config_text).unwrap();
        assert_eq!(config.hosts[0].local_forward, Some("8080 localhost:80".to_string()));
        assert_eq!(config.hosts[0].remote_forward, Some("9090 localhost:90".to_string()));
        assert_eq!(config.hosts[0].dynamic_forward, Some("1080".to_string()));
    }

    #[test]
    fn test_parse_extra_options() {
        let config_text = r#"
Host custom
    HostName custom.com
    CustomOption customvalue
    UnknownKey unknownvalue
"#;

        let config = parse_ssh_config(config_text).unwrap();
        assert_eq!(config.hosts[0].extra_options.get("CustomOption"), Some(&"customvalue".to_string()));
        assert_eq!(config.hosts[0].extra_options.get("UnknownKey"), Some(&"unknownvalue".to_string()));
    }

    #[test]
    fn test_parse_yes_no() {
        assert_eq!(parse_yes_no("yes"), Some(true));
        assert_eq!(parse_yes_no("Yes"), Some(true));
        assert_eq!(parse_yes_no("true"), Some(true));
        assert_eq!(parse_yes_no("no"), Some(false));
        assert_eq!(parse_yes_no("No"), Some(false));
        assert_eq!(parse_yes_no("false"), Some(false));
        assert_eq!(parse_yes_no("invalid"), None);
    }

    #[test]
    fn test_parse_empty_config() {
        let config = parse_ssh_config("").unwrap();
        assert_eq!(config.hosts.len(), 0);
        assert_eq!(config.global_options.len(), 0);
    }

    #[test]
    fn test_parse_comments_only() {
        let config_text = r#"
# Comment line 1
# Comment line 2
"#;

        let config = parse_ssh_config(config_text).unwrap();
        assert_eq!(config.hosts.len(), 0);
    }
}
