use ssh_at_core::keys::{KeyInfo, KeyType};

#[tauri::command]
pub async fn scan_ssh_keys() -> Result<Vec<KeyInfo>, String> {
    eprintln!("[COMMAND] scan_ssh_keys called");

    // Spawn a thread with large stack directly (not through tokio spawn_blocking)
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024) // 64MB stack
        .spawn(move || {
            eprintln!("[COMMAND] In 64MB stack thread");
            let result = ssh_at_core::keys::scan_keys_sync();
            tx.send(result).ok();
        })
        .map_err(|e| format!("Failed to spawn thread: {}", e))?;

    match rx.recv() {
        Ok(Ok(keys)) => {
            eprintln!("[COMMAND] Successfully scanned {} keys", keys.len());
            Ok(keys)
        }
        Ok(Err(e)) => {
            eprintln!("[COMMAND] scan_keys error: {}", e);
            Err(e.to_string())
        }
        Err(e) => {
            eprintln!("[COMMAND] channel recv error: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn get_key_fingerprint(path: String) -> Result<String, String> {
    ssh_at_core::keys::get_fingerprint(&path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_ssh_key(
    key_type: KeyType,
    name: String,
    comment: Option<String>,
    passphrase: Option<String>,
    bits: Option<u32>,
) -> Result<(), String> {
    eprintln!("[COMMAND] generate_ssh_key called");
    eprintln!("[COMMAND]   key_type: {:?}", key_type);
    eprintln!("[COMMAND]   name: {}", name);
    eprintln!("[COMMAND]   comment: {:?}", comment);
    eprintln!("[COMMAND]   passphrase: {:?}", passphrase.as_ref().map(|_| "***"));
    eprintln!("[COMMAND]   bits: {:?}", bits);

    // Validate name to prevent path traversal
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Key name cannot contain path separators or ..".to_string());
    }
    if name.trim().is_empty() {
        return Err("Key name cannot be empty".to_string());
    }

    // Create ~/.ssh-at/creds/ directory if it doesn't exist
    let home = dirs::home_dir()
        .ok_or("Failed to get home directory")?;
    let creds_dir = home.join(".ssh-at").join("creds");

    if !creds_dir.exists() {
        std::fs::create_dir_all(&creds_dir)
            .map_err(|e| format!("Failed to create directory {}: {}", creds_dir.display(), e))?;
    }

    // Build full path
    let path = creds_dir.join(&name);
    let path_str = path.to_str().ok_or("Invalid path")?;

    eprintln!("[COMMAND]   full path: {}", path_str);

    let result = ssh_at_core::keys::generate_key(key_type, path_str, comment.as_deref(), passphrase.as_deref(), bits)
        .await
        .map_err(|e| e.to_string());

    match &result {
        Ok(_) => eprintln!("[COMMAND] generate_ssh_key succeeded"),
        Err(e) => eprintln!("[COMMAND] generate_ssh_key failed: {}", e),
    }

    result
}

#[tauri::command]
pub async fn delete_ssh_key(path: String) -> Result<(), String> {
    eprintln!("[COMMAND] delete_ssh_key called");
    eprintln!("[COMMAND]   path: {}", path);

    // Validate path exists and is a file
    let private_key_path = std::path::Path::new(&path);
    if !private_key_path.exists() {
        return Err(format!("Private key not found: {}", path));
    }
    if !private_key_path.is_file() {
        return Err(format!("Path is not a file: {}", path));
    }

    // Delete private key
    std::fs::remove_file(private_key_path)
        .map_err(|e| format!("Failed to delete private key {}: {}", path, e))?;
    eprintln!("[COMMAND]   Deleted private key: {}", path);

    // Delete public key if exists
    let public_key_path = format!("{}.pub", path);
    if std::path::Path::new(&public_key_path).exists() {
        std::fs::remove_file(&public_key_path)
            .map_err(|e| format!("Failed to delete public key {}: {}", public_key_path, e))?;
        eprintln!("[COMMAND]   Deleted public key: {}", public_key_path);
    } else {
        eprintln!("[COMMAND]   Public key not found (skipped): {}", public_key_path);
    }

    eprintln!("[COMMAND] delete_ssh_key succeeded");
    Ok(())
}

#[tauri::command]
pub async fn read_public_key(private_key_path: String) -> Result<String, String> {
    let public_key_path = format!("{}.pub", private_key_path);
    let path = std::path::Path::new(&public_key_path);

    if !path.exists() {
        return Err(format!("Public key not found: {}", public_key_path));
    }

    std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read public key {}: {}", public_key_path, e))
}
