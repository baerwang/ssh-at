use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    pub path: PathBuf,
    pub key_type: KeyType,
    pub fingerprint: Option<String>,
    pub comment: Option<String>,
    pub size: Option<u32>,
    pub created: Option<String>,
    pub is_encrypted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyType {
    RSA,
    Ed25519,
    ECDSA,
    DSA,
    Unknown,
}

impl std::fmt::Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyType::RSA => write!(f, "RSA"),
            KeyType::Ed25519 => write!(f, "Ed25519"),
            KeyType::ECDSA => write!(f, "ECDSA"),
            KeyType::DSA => write!(f, "DSA"),
            KeyType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_type_display() {
        assert_eq!(KeyType::RSA.to_string(), "RSA");
        assert_eq!(KeyType::Ed25519.to_string(), "Ed25519");
        assert_eq!(KeyType::ECDSA.to_string(), "ECDSA");
        assert_eq!(KeyType::DSA.to_string(), "DSA");
        assert_eq!(KeyType::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_key_type_partial_eq() {
        assert_eq!(KeyType::RSA, KeyType::RSA);
        assert_ne!(KeyType::RSA, KeyType::Ed25519);
        assert_eq!(KeyType::Unknown, KeyType::Unknown);
    }
}
