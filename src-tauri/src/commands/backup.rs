use ssh_at_storage::backup::BackupInfo;

#[tauri::command]
pub async fn list_backups() -> Result<Vec<BackupInfo>, String> {
    ssh_at_storage::backup::list_backups()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restore_backup(backup_id: i64) -> Result<(), String> {
    ssh_at_storage::backup::restore_backup(backup_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_backup(backup_id: i64) -> Result<(), String> {
    ssh_at_storage::backup::delete_backup(backup_id)
        .await
        .map_err(|e| e.to_string())
}
