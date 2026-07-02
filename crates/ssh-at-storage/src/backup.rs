use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::PathBuf;
use tokio::fs;
use sha2::Digest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: i64,
    pub timestamp: String,
    pub file_path: String,
    pub config_hash: String,
    pub host_count: i32,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppSettings {
    pub auto_backup: bool,
    pub backup_limit: u32,
    pub confirm_delete: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_backup: true,
            backup_limit: 10,
            confirm_delete: true,
        }
    }
}

/// Get backup directory
fn get_backup_dir_impl(home: Option<&std::path::Path>) -> PathBuf {
    home.map(|h| h.join(".ssh-at").join("backups"))
        .or_else(|| dirs::home_dir().map(|h| h.join(".ssh-at").join("backups")))
        .unwrap_or_else(|| PathBuf::from(".ssh-at/backups"))
}

/// Load settings from ~/.ssh-at/settings.json
async fn load_settings() -> AppSettings {
    load_settings_impl(None).await
}

async fn load_settings_impl(home: Option<&std::path::Path>) -> AppSettings {
    let settings_path = home.map(|h| h.join(".ssh-at").join("settings.json"))
        .or_else(|| dirs::home_dir().map(|h| h.join(".ssh-at").join("settings.json")))
        .unwrap_or_else(|| PathBuf::from(".ssh-at/settings.json"));

    if let Ok(content) = fs::read_to_string(&settings_path).await {
        if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
            return settings;
        }
    }

    AppSettings::default()
}

/// Create a backup of the SSH config
pub async fn create_backup(config_content: &str) -> Result<BackupInfo> {
    let settings = load_settings().await;

    // 如果 auto_backup 关闭，直接返回空的 BackupInfo
    if !settings.auto_backup {
        return Ok(BackupInfo {
            id: 0,
            timestamp: String::new(),
            file_path: String::new(),
            config_hash: String::new(),
            host_count: 0,
            size_bytes: 0,
        });
    }

    let limit = settings.backup_limit as usize;
    create_backup_with_limit_impl(config_content, limit, None).await
}

/// Create a backup of the SSH config with custom limit
pub async fn create_backup_with_limit(config_content: &str, limit: usize) -> Result<BackupInfo> {
    create_backup_with_limit_impl(config_content, limit, None).await
}

async fn create_backup_with_limit_impl(config_content: &str, limit: usize, home: Option<&std::path::Path>) -> Result<BackupInfo> {
    let backup_dir = get_backup_dir_impl(home);
    fs::create_dir_all(&backup_dir).await?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_file = backup_dir.join(format!("{}.config", timestamp));

    // Write backup file
    fs::write(&backup_file, config_content).await?;

    // Calculate hash
    let hash = format!("{:x}", sha2::Sha256::digest(config_content.as_bytes()));

    // Count hosts (simple count of "Host " lines)
    let host_count = config_content.lines()
        .filter(|line| line.trim().starts_with("Host "))
        .count() as i32;

    let size_bytes = config_content.len() as i64;

    let backup_info = BackupInfo {
        id: chrono::Local::now().timestamp(),
        timestamp: timestamp.clone(),
        file_path: backup_file.to_string_lossy().to_string(),
        config_hash: hash,
        host_count,
        size_bytes,
    };

    // Clean up old backups with custom limit
    cleanup_old_backups_with_limit_impl(limit, home).await?;

    Ok(backup_info)
}

/// List all backups
pub async fn list_backups() -> Result<Vec<BackupInfo>> {
    list_backups_impl(None).await
}

async fn list_backups_impl(home: Option<&std::path::Path>) -> Result<Vec<BackupInfo>> {
    let backup_dir = get_backup_dir_impl(home);

    if !backup_dir.exists() {
        return Ok(vec![]);
    }

    let mut backups = Vec::new();
    let mut entries = fs::read_dir(&backup_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "config") {
            if let Ok(content) = fs::read_to_string(&path).await {
                let metadata = fs::metadata(&path).await?;
                let timestamp = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let hash = format!("{:x}", sha2::Sha256::digest(content.as_bytes()));
                let host_count = content.lines()
                    .filter(|line| line.trim().starts_with("Host "))
                    .count() as i32;

                backups.push(BackupInfo {
                    id: metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
                    timestamp,
                    file_path: path.to_string_lossy().to_string(),
                    config_hash: hash,
                    host_count,
                    size_bytes: metadata.len() as i64,
                });
            }
        }
    }

    // Sort by timestamp descending
    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(backups)
}

/// Restore backup by copying to ~/.ssh/config
pub async fn restore_backup(backup_id: i64) -> Result<()> {
    let backups = list_backups().await?;

    let backup = backups.iter()
        .find(|b| b.id == backup_id)
        .ok_or_else(|| anyhow::anyhow!("Backup not found"))?;

    // Read backup content first before any operations
    let backup_content = fs::read_to_string(&backup.file_path).await?;

    let config_path = dirs::home_dir()
        .map(|h| h.join(".ssh").join("config"))
        .unwrap_or_else(|| PathBuf::from(".ssh/config"));

    // Create backup of current config before restoring
    if config_path.exists() {
        let current_content = fs::read_to_string(&config_path).await?;
        create_backup(&current_content).await?;
    }

    // Restore from backup content (not file path, as cleanup may have deleted it)
    fs::write(&config_path, backup_content).await?;

    Ok(())
}

/// Delete a backup
pub async fn delete_backup(backup_id: i64) -> Result<()> {
    let backups = list_backups().await?;

    let backup = backups.iter()
        .find(|b| b.id == backup_id)
        .ok_or_else(|| anyhow::anyhow!("Backup not found"))?;

    fs::remove_file(&backup.file_path).await?;

    Ok(())
}

/// Clean up old backups, keeping only the last N backups
async fn cleanup_old_backups_with_limit_impl(limit: usize, home: Option<&std::path::Path>) -> Result<()> {
    let mut backups = list_backups_impl(home).await?;

    if backups.len() > limit {
        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        for backup in backups.iter().skip(limit) {
            let _ = fs::remove_file(&backup.file_path).await;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn setup_test_env() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
        let guard = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let temp_dir = TempDir::new().unwrap();

        // Create settings.json with default values for tests
        let settings_dir = temp_dir.path().join(".ssh-at");
        std::fs::create_dir_all(&settings_dir).unwrap();
        let settings_path = settings_dir.join("settings.json");
        let settings = AppSettings {
            auto_backup: true,
            backup_limit: 10,
            confirm_delete: true,
        };
        std::fs::write(&settings_path, serde_json::to_string(&settings).unwrap()).unwrap();

        (temp_dir, guard)
    }

    #[tokio::test]
    async fn test_create_backup() {
        let (temp_dir, _guard) = setup_test_env();
        let temp_path = temp_dir.path().to_path_buf();
        drop(_guard);

        let config_content = "Host test\n    HostName example.com\n    User admin\n";
        let backup = create_backup_with_limit_impl(config_content, 10, Some(&temp_path)).await.unwrap();

        assert_eq!(backup.host_count, 1);
        assert_eq!(backup.size_bytes, config_content.len() as i64);
        assert!(!backup.config_hash.is_empty());
        assert!(PathBuf::from(&backup.file_path).exists());
    }

    #[tokio::test]
    async fn test_list_backups_empty() {
        let (temp_dir, _guard) = setup_test_env();
        let temp_path = temp_dir.path().to_path_buf();
        drop(_guard);

        let backups = list_backups_impl(Some(&temp_path)).await.unwrap();
        assert_eq!(backups.len(), 0);
    }

    #[tokio::test]
    async fn test_list_backups_multiple() {
        let (temp_dir, _guard) = setup_test_env();
        let temp_path = temp_dir.path().to_path_buf();
        drop(_guard);

        let backup_dir = get_backup_dir_impl(Some(&temp_path));
        fs::create_dir_all(&backup_dir).await.unwrap();

        let timestamp1 = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_file1 = backup_dir.join(format!("{}.config", timestamp1));
        fs::write(&backup_file1, "Host test1\n    HostName example.com\n").await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let timestamp2 = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_file2 = backup_dir.join(format!("{}.config", timestamp2));
        fs::write(&backup_file2, "Host test2\n    HostName example.org\n").await.unwrap();

        let backups = list_backups_impl(Some(&temp_path)).await.unwrap();

        assert_eq!(backups.len(), 2);
        assert!(backups[0].timestamp > backups[1].timestamp);
    }

    #[tokio::test]
    async fn test_backup_hash_uniqueness() {
        let (temp_dir, _guard) = setup_test_env();
        let temp_path = temp_dir.path().to_path_buf();
        drop(_guard);

        let backup_dir = get_backup_dir_impl(Some(&temp_path));
        fs::create_dir_all(&backup_dir).await.unwrap();

        let backup1 = create_backup_with_limit_impl("Host test1\n", 10, Some(&temp_path)).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let backup2 = create_backup_with_limit_impl("Host test2\n", 10, Some(&temp_path)).await.unwrap();

        assert_ne!(backup1.config_hash, backup2.config_hash);
    }

    #[tokio::test]
    async fn test_delete_backup() {
        let (temp_dir, _guard) = setup_test_env();
        let temp_path = temp_dir.path().to_path_buf();
        drop(_guard);

        let backup = create_backup_with_limit_impl("Host test\n", 10, Some(&temp_path)).await.unwrap();

        assert!(PathBuf::from(&backup.file_path).exists());

        let backups_before = list_backups_impl(Some(&temp_path)).await.unwrap();
        assert_eq!(backups_before.len(), 1);

        fs::remove_file(&backup.file_path).await.unwrap();

        let backups_after = list_backups_impl(Some(&temp_path)).await.unwrap();
        assert_eq!(backups_after.len(), 0);
    }

    #[tokio::test]
    async fn test_restore_backup() {
        let (temp_dir, _guard) = setup_test_env();
        let temp_path = temp_dir.path().to_path_buf();
        drop(_guard);

        let ssh_dir = temp_path.join(".ssh");
        tokio::fs::create_dir_all(&ssh_dir).await.unwrap();
        let config_path = ssh_dir.join("config");

        let original_content = "Host original\n    HostName original.com\n";
        tokio::fs::write(&config_path, original_content).await.unwrap();

        let backup_dir = get_backup_dir_impl(Some(&temp_path));
        fs::create_dir_all(&backup_dir).await.unwrap();

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_file = backup_dir.join(format!("{}.config", timestamp));
        fs::write(&backup_file, original_content).await.unwrap();

        let backups = list_backups_impl(Some(&temp_path)).await.unwrap();
        assert!(!backups.is_empty());

        let new_content = "Host modified\n    HostName modified.com\n";
        tokio::fs::write(&config_path, new_content).await.unwrap();

        let backup_content = fs::read_to_string(&backup_file).await.unwrap();
        fs::write(&config_path, backup_content).await.unwrap();

        let restored_content = tokio::fs::read_to_string(&config_path).await.unwrap();
        assert_eq!(restored_content, original_content);
    }

    #[tokio::test]
    async fn test_cleanup_old_backups() {
        let (temp_dir, _guard) = setup_test_env();
        let temp_path = temp_dir.path().to_path_buf();
        drop(_guard);

        for _i in 0..15 {
            create_backup_with_limit_impl("Host test\n", 10, Some(&temp_path)).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let backups = list_backups_impl(Some(&temp_path)).await.unwrap();
        assert!(backups.len() <= 10);
    }

    #[tokio::test]
    async fn test_backup_host_count() {
        let (temp_dir, _guard) = setup_test_env();
        let temp_path = temp_dir.path().to_path_buf();
        drop(_guard);

        let config = "Host server1\n    HostName s1.com\n\nHost server2\n    HostName s2.com\n\nHost server3\n    HostName s3.com\n";
        let backup = create_backup_with_limit_impl(config, 10, Some(&temp_path)).await.unwrap();

        assert_eq!(backup.host_count, 3);
    }
}
