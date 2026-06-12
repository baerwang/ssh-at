use super::types::{KeyInfo, KeyType};
use super::generator::get_fingerprint;
use crate::config::load_config;
use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::fs;

/// Scan SSH keys (synchronous version, prioritizes default keys to avoid config parsing stack overflow)
pub fn scan_keys_sync() -> Result<Vec<KeyInfo>> {
    let mut scanned_paths = HashSet::new();
    let mut keys = Vec::new();

    // First: scan default key locations (always works, no config dependency)
    match scan_default_keys_sync() {
        Ok(default_keys) => {
            for key in default_keys {
                if scanned_paths.insert(key.path.clone()) {
                    keys.push(key);
                }
            }
        }
        Err(e) => {
            eprintln!("[SCANNER] Failed to scan default keys: {}", e);
        }
    }

    // Second: try to read config for additional IdentityFile entries (best-effort, may fail)
    // Note: We skip this for now to avoid stack overflow from recursive Include parsing
    // TODO: Implement shallow config reading (no Include recursion) for additional keys

    Ok(keys)
}

/// Scan SSH keys referenced in ~/.ssh/config
/// Only scans keys that are actually used by configured hosts
pub async fn scan_keys() -> Result<Vec<KeyInfo>> {
    let config = load_config().await?;
    let mut scanned_paths = HashSet::new();
    let mut keys = Vec::new();

    // Collect all unique IdentityFile paths from config
    for host in &config.hosts {
        if let Some(ref identity_file) = host.identity_file {
            let path = expand_tilde_path(identity_file);
            if scanned_paths.insert(path.clone()) && path.exists() {
                if let Ok(key_info) = scan_key_file(&path).await {
                    keys.push(key_info);
                }
            }
        }
    }

    // If no keys found in config, fall back to scanning common default keys
    if keys.is_empty() {
        keys = scan_default_keys().await?;
    }

    Ok(keys)
}

/// Scan a single key file and extract metadata (synchronous version)
fn scan_key_file_sync(path: &PathBuf) -> Result<KeyInfo> {
    let content = std::fs::read_to_string(path)?;

    if !is_private_key(&content) {
        anyhow::bail!("Not a valid private key file");
    }

    let key_type = detect_key_type(&content);
    let is_encrypted = content.contains("ENCRYPTED");

    // Note: fingerprint extraction is skipped in sync version (requires ssh-keygen subprocess)
    let fingerprint = None;

    Ok(KeyInfo {
        path: path.clone(),
        key_type,
        fingerprint,
        comment: None,
        size: None,
        created: get_file_created_sync(path),
        is_encrypted,
    })
}

/// Scan a single key file and extract metadata
async fn scan_key_file(path: &PathBuf) -> Result<KeyInfo> {
    let content = fs::read_to_string(path).await?;

    if !is_private_key(&content) {
        anyhow::bail!("Not a valid private key file");
    }

    let key_type = detect_key_type(&content);
    let is_encrypted = content.contains("ENCRYPTED");

    // Extract fingerprint using ssh-keygen
    let fingerprint = get_fingerprint(path.to_str().unwrap_or("")).await.ok();

    Ok(KeyInfo {
        path: path.clone(),
        key_type,
        fingerprint,
        comment: None,
        size: None,
        created: get_file_created(path).await,
        is_encrypted,
    })
}

/// Scan default SSH key locations (~/.ssh/ and ~/.ssh-at/creds/) (synchronous version)
fn scan_default_keys_sync() -> Result<Vec<KeyInfo>> {
    let mut keys = Vec::new();

    // Scan both directories
    let ssh_dir = get_ssh_dir();
    let ssh_at_dir = get_ssh_at_dir();

    for dir in [ssh_dir, ssh_at_dir] {
        if !dir.exists() {
            continue;
        }

        // Read all files in directory
        match std::fs::read_dir(&dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();

                    // Skip directories and .pub files
                    if path.is_dir() || path.extension().map_or(false, |ext| ext == "pub") {
                        continue;
                    }

                    // Try to scan as key file
                    if let Ok(key_info) = scan_key_file_sync(&path) {
                        keys.push(key_info);
                    }
                }
            }
            Err(e) => {
                eprintln!("[SCANNER] Failed to read directory {:?}: {}", dir, e);
            }
        }
    }

    Ok(keys)
}

/// Scan default SSH key locations (~/.ssh/ and ~/.ssh-at/creds/)
async fn scan_default_keys() -> Result<Vec<KeyInfo>> {
    let mut keys = Vec::new();

    // Scan both directories
    let ssh_dir = get_ssh_dir();
    let ssh_at_dir = get_ssh_at_dir();

    for dir in [ssh_dir, ssh_at_dir] {
        if !dir.exists() {
            continue;
        }

        // Read all files in directory
        match fs::read_dir(&dir).await {
            Ok(mut entries) => {
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();

                    // Skip directories and .pub files
                    if path.is_dir() || path.extension().map_or(false, |ext| ext == "pub") {
                        continue;
                    }

                    // Try to scan as key file
                    if let Ok(key_info) = scan_key_file(&path).await {
                        keys.push(key_info);
                    }
                }
            }
            Err(e) => {
                eprintln!("[SCANNER] Failed to read directory {:?}: {}", dir, e);
            }
        }
    }

    Ok(keys)
}

/// Expand ~/ to actual home directory path
fn expand_tilde_path(path: &PathBuf) -> PathBuf {
    if let Some(path_str) = path.to_str() {
        if path_str.starts_with("~/") {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            return PathBuf::from(home).join(&path_str[2..]);
        }
    }
    path.clone()
}

fn get_ssh_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".ssh")
}

fn get_ssh_at_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".ssh-at").join("creds")
}

fn is_private_key(content: &str) -> bool {
    content.contains("BEGIN") && content.contains("PRIVATE KEY")
}

fn detect_key_type(content: &str) -> KeyType {
    if content.contains("RSA PRIVATE KEY") {
        KeyType::RSA
    } else if content.contains("OPENSSH PRIVATE KEY") {
        KeyType::Ed25519
    } else if content.contains("EC PRIVATE KEY") {
        KeyType::ECDSA
    } else if content.contains("DSA PRIVATE KEY") {
        KeyType::DSA
    } else {
        KeyType::Unknown
    }
}

fn get_file_created_sync(path: &PathBuf) -> Option<String> {
    if let Ok(metadata) = std::fs::metadata(path) {
        if let Ok(created) = metadata.modified() {
            let datetime: chrono::DateTime<chrono::Local> = created.into();
            return Some(datetime.format("%Y-%m-%d %H:%M:%S").to_string());
        }
    }
    None
}

async fn get_file_created(path: &PathBuf) -> Option<String> {
    if let Ok(metadata) = fs::metadata(path).await {
        if let Ok(created) = metadata.modified() {
            let datetime: chrono::DateTime<chrono::Local> = created.into();
            return Some(datetime.format("%Y-%m-%d %H:%M:%S").to_string());
        }
    }
    None
}

/// Get all unique key paths referenced in SSH config
pub async fn get_config_key_paths() -> Result<Vec<PathBuf>> {
    let config = load_config().await?;
    let mut paths = HashSet::new();

    for host in &config.hosts {
        if let Some(ref identity_file) = host.identity_file {
            let expanded = expand_tilde_path(identity_file);
            if expanded.exists() {
                paths.insert(expanded);
            }
        }
    }

    Ok(paths.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_config_with_keys(temp_dir: &TempDir) -> PathBuf {
        let ssh_dir = temp_dir.path().join(".ssh");
        tokio::fs::create_dir_all(&ssh_dir).await.unwrap();

        // Create test keys
        let rsa_key = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        let rsa_path = ssh_dir.join("id_rsa");
        tokio::fs::write(&rsa_path, rsa_key).await.unwrap();

        let ed25519_key = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAA...\n-----END OPENSSH PRIVATE KEY-----";
        let ed25519_path = ssh_dir.join("id_ed25519");
        tokio::fs::write(&ed25519_path, ed25519_key).await.unwrap();

        // Create config referencing these keys
        let config_content = format!(
            "Host server1\n  HostName example.com\n  IdentityFile {}\n\nHost server2\n  HostName test.com\n  IdentityFile {}\n",
            rsa_path.display(),
            ed25519_path.display()
        );
        let config_path = ssh_dir.join("config");
        tokio::fs::write(&config_path, config_content).await.unwrap();

        std::env::set_var("HOME", temp_dir.path());
        ssh_dir
    }

    #[tokio::test]
    async fn test_scan_keys_from_config() {
        let temp_dir = TempDir::new().unwrap();
        create_test_config_with_keys(&temp_dir).await;

        let keys = scan_keys().await.unwrap();
        assert_eq!(keys.len(), 2, "Should find 2 keys referenced in config");
    }

    #[tokio::test]
    async fn test_scan_default_keys_when_no_config() {
        let temp_dir = TempDir::new().unwrap();
        let ssh_dir = temp_dir.path().join(".ssh");
        tokio::fs::create_dir_all(&ssh_dir).await.unwrap();

        // Create default key without config
        let rsa_key = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        tokio::fs::write(ssh_dir.join("id_rsa"), rsa_key).await.unwrap();

        std::env::set_var("HOME", temp_dir.path());

        let keys = scan_keys().await.unwrap();
        assert_eq!(keys.len(), 1, "Should fall back to scanning default keys");
        assert_eq!(keys[0].key_type, KeyType::RSA);
    }

    #[tokio::test]
    async fn test_expand_tilde_path() {
        std::env::set_var("HOME", "/home/testuser");

        let tilde_path = PathBuf::from("~/.ssh/id_rsa");
        let expanded = expand_tilde_path(&tilde_path);

        assert_eq!(expanded, PathBuf::from("/home/testuser/.ssh/id_rsa"));
    }

    #[tokio::test]
    async fn test_detect_rsa_key() {
        let content = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        assert!(is_private_key(content));
        assert_eq!(detect_key_type(content), KeyType::RSA);
    }

    #[tokio::test]
    async fn test_detect_ed25519_key() {
        let content = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAA...\n-----END OPENSSH PRIVATE KEY-----";
        assert!(is_private_key(content));
        assert_eq!(detect_key_type(content), KeyType::Ed25519);
    }

    #[tokio::test]
    async fn test_detect_ecdsa_key() {
        let content = "-----BEGIN EC PRIVATE KEY-----\nMHcCAQEEIIGQjmqj...\n-----END EC PRIVATE KEY-----";
        assert!(is_private_key(content));
        assert_eq!(detect_key_type(content), KeyType::ECDSA);
    }

    #[tokio::test]
    async fn test_detect_encrypted_key() {
        let content = "-----BEGIN RSA PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\nDEK-Info: AES-128-CBC,ABCD...\n-----END RSA PRIVATE KEY-----";
        assert!(content.contains("ENCRYPTED"));
    }

    #[tokio::test]
    async fn test_scan_keys_deduplicates_paths() {
        let temp_dir = TempDir::new().unwrap();
        let ssh_dir = temp_dir.path().join(".ssh");
        tokio::fs::create_dir_all(&ssh_dir).await.unwrap();

        let rsa_key = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        let rsa_path = ssh_dir.join("id_rsa");
        tokio::fs::write(&rsa_path, rsa_key).await.unwrap();

        // Config with duplicate IdentityFile references
        let config_content = format!(
            "Host server1\n  IdentityFile {}\nHost server2\n  IdentityFile {}\n",
            rsa_path.display(),
            rsa_path.display()
        );
        let config_path = ssh_dir.join("config");
        tokio::fs::write(&config_path, config_content).await.unwrap();

        std::env::set_var("HOME", temp_dir.path());

        let keys = scan_keys().await.unwrap();
        assert_eq!(keys.len(), 1, "Should deduplicate same key path");
    }

    #[tokio::test]
    async fn test_detect_dsa_key() {
        let content = "-----BEGIN DSA PRIVATE KEY-----\nMIIBuwIBAAKBgQD...\n-----END DSA PRIVATE KEY-----";
        assert!(is_private_key(content));
        assert_eq!(detect_key_type(content), KeyType::DSA);
    }

    #[tokio::test]
    async fn test_detect_unknown_key() {
        let content = "-----BEGIN UNKNOWN KEY-----\nSomeRandomContent...\n-----END UNKNOWN KEY-----";
        assert_eq!(detect_key_type(content), KeyType::Unknown);
    }

    #[tokio::test]
    async fn test_scan_keys_nonexistent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let non_existent = temp_dir.path().join("nonexistent");

        std::env::set_var("HOME", &non_existent);

        let keys = scan_keys().await.unwrap();
        assert_eq!(keys.len(), 0);
    }

    #[tokio::test]
    async fn test_get_file_created_nonexistent_file() {
        let nonexistent_path = PathBuf::from("/nonexistent/path/to/file");
        let result = get_file_created(&nonexistent_path).await;
        assert_eq!(result, None);
    }
}
