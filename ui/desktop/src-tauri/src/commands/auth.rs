//! Authentication-related commands (secure credential storage)

use serde::{Deserialize, Serialize};

const SERVICE_NAME: &str = "io.gausstwin.desktop";

/// Stored credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCredentials {
    pub email: String,
    pub access_token: String,
    pub refresh_token: String,
}

/// Get stored credentials from system keychain
#[tauri::command]
pub async fn get_stored_credentials() -> Result<Option<StoredCredentials>, String> {
    let entry = keyring::Entry::new(SERVICE_NAME, "credentials")
        .map_err(|e| format!("Failed to access keychain: {}", e))?;

    match entry.get_password() {
        Ok(json) => {
            let creds: StoredCredentials = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to parse credentials: {}", e))?;
            Ok(Some(creds))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to get credentials: {}", e)),
    }
}

/// Store credentials in system keychain
#[tauri::command]
pub async fn store_credentials(credentials: StoredCredentials) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, "credentials")
        .map_err(|e| format!("Failed to access keychain: {}", e))?;

    let json = serde_json::to_string(&credentials)
        .map_err(|e| format!("Failed to serialize credentials: {}", e))?;

    entry.set_password(&json)
        .map_err(|e| format!("Failed to store credentials: {}", e))
}

/// Delete stored credentials
#[tauri::command]
pub async fn delete_credentials() -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, "credentials")
        .map_err(|e| format!("Failed to access keychain: {}", e))?;

    match entry.delete_password() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
        Err(e) => Err(format!("Failed to delete credentials: {}", e)),
    }
}
