use super::types::KeyType;
use anyhow::Result;
use tokio::process::Command;

/// Generate SSH key using ssh-keygen
pub async fn generate_key(
    key_type: KeyType,
    path: &str,
    comment: Option<&str>,
    passphrase: Option<&str>,
    bits: Option<u32>,
) -> Result<()> {
    let type_arg = match key_type {
        KeyType::RSA => "rsa",
        KeyType::Ed25519 => "ed25519",
        KeyType::ECDSA => "ecdsa",
        KeyType::DSA => "dsa",
        KeyType::Unknown => anyhow::bail!("Cannot generate key of unknown type"),
    };

    let mut cmd = Command::new("ssh-keygen");
    cmd.arg("-t").arg(type_arg)
       .arg("-f").arg(path)
       .arg("-N").arg(passphrase.unwrap_or(""));

    if let Some(bits) = bits {
        cmd.arg("-b").arg(bits.to_string());
    }

    if let Some(comment) = comment {
        cmd.arg("-C").arg(comment);
    }

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ssh-keygen failed: {}", stderr);
    }

    // Set private key file permissions to 600 on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        tokio::fs::set_permissions(path, perms).await?;
    }

    Ok(())
}

/// Get fingerprint of an SSH key
pub async fn get_fingerprint(path: &str) -> Result<String> {
    let output = Command::new("ssh-keygen")
        .arg("-lf")
        .arg(path)
        .arg("-E")
        .arg("sha256")
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ssh-keygen failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse output: "2048 SHA256:... user@host (RSA)"
    let parts: Vec<&str> = stdout.split_whitespace().collect();
    if parts.len() >= 2 {
        Ok(parts[1].to_string())
    } else {
        anyhow::bail!("Failed to parse fingerprint from ssh-keygen output")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_generate_rsa_key() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("test_rsa");
        let key_path_str = key_path.to_str().unwrap();

        let result = generate_key(KeyType::RSA, key_path_str, None, None, None).await;
        assert!(result.is_ok());
        assert!(key_path.exists());

        // Check permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = tokio::fs::metadata(&key_path).await.unwrap();
            let mode = metadata.permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "Private key should have 600 permissions");
        }
    }

    #[tokio::test]
    async fn test_generate_ed25519_key() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("test_ed25519");
        let key_path_str = key_path.to_str().unwrap();

        let result = generate_key(KeyType::Ed25519, key_path_str, None, None, None).await;
        assert!(result.is_ok());
        assert!(key_path.exists());
    }

    #[tokio::test]
    async fn test_generate_ecdsa_key() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("test_ecdsa");
        let key_path_str = key_path.to_str().unwrap();

        let result = generate_key(KeyType::ECDSA, key_path_str, None, None, None).await;
        assert!(result.is_ok());
        assert!(key_path.exists());
    }

    #[tokio::test]
    async fn test_generate_with_passphrase() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("test_passphrase");
        let key_path_str = key_path.to_str().unwrap();

        let result = generate_key(
            KeyType::Ed25519,
            key_path_str,
            None,
            Some("test_password"),
            None,
        )
        .await;
        assert!(result.is_ok());
        assert!(key_path.exists());

        // Verify key is encrypted by trying to read it with empty passphrase
        let output = Command::new("ssh-keygen")
            .arg("-y")
            .arg("-f")
            .arg(key_path_str)
            .arg("-P")
            .arg("")
            .output()
            .await
            .unwrap();

        // Should fail with "incorrect passphrase" error
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("passphrase") || stderr.contains("decrypt"));
    }

    #[tokio::test]
    async fn test_generate_with_comment() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("test_comment");
        let key_path_str = key_path.to_str().unwrap();

        let result = generate_key(
            KeyType::Ed25519,
            key_path_str,
            Some("test@example.com"),
            None,
            None,
        )
        .await;
        assert!(result.is_ok());
        assert!(key_path.exists());

        let pub_key_path = format!("{}.pub", key_path_str);
        let pub_content = tokio::fs::read_to_string(&pub_key_path).await.unwrap();
        assert!(pub_content.contains("test@example.com"));
    }

    #[tokio::test]
    async fn test_generate_unknown_type_fails() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("test_unknown");
        let key_path_str = key_path.to_str().unwrap();

        let result = generate_key(KeyType::Unknown, key_path_str, None, None, None).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot generate key of unknown type"));
    }

    #[tokio::test]
    async fn test_get_fingerprint() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("test_fingerprint");
        let key_path_str = key_path.to_str().unwrap();

        generate_key(KeyType::Ed25519, key_path_str, None, None, None)
            .await
            .unwrap();

        let fingerprint = get_fingerprint(key_path_str).await.unwrap();
        assert!(fingerprint.starts_with("SHA256:"));
        assert!(fingerprint.len() > 10);
    }

    #[tokio::test]
    async fn test_get_fingerprint_nonexistent_file() {
        let result = get_fingerprint("/nonexistent/path/to/key").await;
        assert!(result.is_err());
    }
}
