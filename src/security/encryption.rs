// src/security/encryption.rs
//! Field-level AES-256-GCM encryption with key rotation.
//!
//! Sensitive memory payloads can be encrypted at rest. Each ciphertext records
//! the **key version** used so that keys can be rotated without re-encrypting
//! existing data: new writes use the current key, while reads transparently
//! select the matching historical key by version. A fresh random 96-bit nonce
//! is generated per encryption (never reused with the same key).
//!
//! Compiled only with the `encryption` feature.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Errors that can arise during encryption / decryption.
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("unknown key version {0}")]
    UnknownKeyVersion(u32),
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed (wrong key or corrupt ciphertext)")]
    Decrypt,
}

/// An encrypted field: the key version, the per-message nonce, and ciphertext.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedField {
    pub key_version: u32,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

/// Encrypts/decrypts fields with a set of versioned 256-bit keys.
///
/// The `current` version is used for all new encryptions; any registered
/// version can still be used for decryption, enabling zero-downtime rotation.
pub struct FieldEncryptor {
    keys: HashMap<u32, Aes256Gcm>,
    current: u32,
}

impl FieldEncryptor {
    /// Create an encryptor with a single key at version `version`.
    ///
    /// `key` must be exactly 32 bytes (AES-256).
    pub fn new(version: u32, key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let mut keys = HashMap::new();
        keys.insert(version, cipher);
        Self { keys, current: version }
    }

    /// Register an additional key version (e.g. an older key for decryption, or
    /// a new key to rotate to). Does not change the current version.
    pub fn add_key(&mut self, version: u32, key: &[u8; 32]) -> &mut Self {
        self.keys
            .insert(version, Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key)));
        self
    }

    /// Rotate so that new encryptions use `version` (which must be registered).
    pub fn set_current(&mut self, version: u32) -> Result<(), EncryptionError> {
        if !self.keys.contains_key(&version) {
            return Err(EncryptionError::UnknownKeyVersion(version));
        }
        self.current = version;
        Ok(())
    }

    pub fn current_version(&self) -> u32 {
        self.current
    }

    /// Encrypt `plaintext` with the current key and a fresh random nonce.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedField, EncryptionError> {
        let cipher = self
            .keys
            .get(&self.current)
            .ok_or(EncryptionError::UnknownKeyVersion(self.current))?;
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| EncryptionError::Encrypt)?;
        Ok(EncryptedField {
            key_version: self.current,
            nonce: nonce_bytes.to_vec(),
            ciphertext,
        })
    }

    /// Decrypt a field using the key version recorded in it.
    pub fn decrypt(&self, field: &EncryptedField) -> Result<Vec<u8>, EncryptionError> {
        let cipher = self
            .keys
            .get(&field.key_version)
            .ok_or(EncryptionError::UnknownKeyVersion(field.key_version))?;
        let nonce = Nonce::from_slice(&field.nonce);
        cipher
            .decrypt(nonce, field.ciphertext.as_ref())
            .map_err(|_| EncryptionError::Decrypt)
    }

    /// Convenience: encrypt a UTF-8 string.
    pub fn encrypt_str(&self, s: &str) -> Result<EncryptedField, EncryptionError> {
        self.encrypt(s.as_bytes())
    }

    /// Convenience: decrypt to a UTF-8 string.
    pub fn decrypt_str(&self, field: &EncryptedField) -> Result<String, EncryptionError> {
        let bytes = self.decrypt(field)?;
        String::from_utf8(bytes).map_err(|_| EncryptionError::Decrypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let enc = FieldEncryptor::new(1, &[7u8; 32]);
        let field = enc.encrypt_str("secret memory").unwrap();
        assert_eq!(field.key_version, 1);
        assert_eq!(field.nonce.len(), 12);
        assert_ne!(field.ciphertext, b"secret memory");
        assert_eq!(enc.decrypt_str(&field).unwrap(), "secret memory");
    }

    #[test]
    fn nonce_is_unique_per_encryption() {
        let enc = FieldEncryptor::new(1, &[1u8; 32]);
        let a = enc.encrypt_str("same").unwrap();
        let b = enc.encrypt_str("same").unwrap();
        assert_ne!(a.nonce, b.nonce);
        assert_ne!(a.ciphertext, b.ciphertext); // GCM is randomised by nonce
    }

    #[test]
    fn rotation_decrypts_old_and_new() {
        let mut enc = FieldEncryptor::new(1, &[1u8; 32]);
        let old = enc.encrypt_str("written under v1").unwrap();

        // Rotate to v2.
        enc.add_key(2, &[2u8; 32]);
        enc.set_current(2).unwrap();
        let new = enc.encrypt_str("written under v2").unwrap();
        assert_eq!(new.key_version, 2);

        // Both decrypt because v1 is still registered.
        assert_eq!(enc.decrypt_str(&old).unwrap(), "written under v1");
        assert_eq!(enc.decrypt_str(&new).unwrap(), "written under v2");
    }

    #[test]
    fn wrong_key_fails() {
        let enc1 = FieldEncryptor::new(1, &[1u8; 32]);
        let enc2 = FieldEncryptor::new(1, &[9u8; 32]);
        let field = enc1.encrypt_str("classified").unwrap();
        assert!(matches!(
            enc2.decrypt(&field),
            Err(EncryptionError::Decrypt)
        ));
    }

    #[test]
    fn unknown_version_fails() {
        let enc = FieldEncryptor::new(1, &[1u8; 32]);
        let field = EncryptedField {
            key_version: 99,
            nonce: vec![0u8; 12],
            ciphertext: vec![0u8; 16],
        };
        assert!(matches!(
            enc.decrypt(&field),
            Err(EncryptionError::UnknownKeyVersion(99))
        ));
    }
}
