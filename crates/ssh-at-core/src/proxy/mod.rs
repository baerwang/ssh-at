// Proxy configuration manager

/// Generate ProxyCommand for local SOCKS5 proxy
pub fn generate_socks5_proxy_command(host: &str, port: u16) -> String {
    format!("nc -x {}:{} %h %p", host, port)
}

/// Validate proxy configuration
pub fn validate_proxy_command(command: &str) -> bool {
    !command.is_empty() && command.contains("%h") && command.contains("%p")
}
