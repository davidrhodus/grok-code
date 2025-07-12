use crate::error::{GrokError, Result};
use keyring::Entry;

const SERVICE_NAME: &str = "grok-code";

/// Manages secure storage of API keys
pub struct KeyStore {
    service: String,
}

impl KeyStore {
    /// Create a new KeyStore instance
    pub fn new() -> Self {
        Self {
            service: SERVICE_NAME.to_string(),
        }
    }

    /// Store an API key securely
    pub fn set_api_key(&self, provider: &str, api_key: &str) -> Result<()> {
        let entry = Entry::new(&self.service, provider)
            .map_err(|e| GrokError::Config(format!("Failed to create keyring entry: {e}")))?;

        entry
            .set_password(api_key)
            .map_err(|e| GrokError::Config(format!("Failed to store API key: {e}")))?;

        Ok(())
    }

    /// Retrieve an API key from secure storage
    pub fn get_api_key(&self, provider: &str) -> Result<String> {
        let entry = Entry::new(&self.service, provider)
            .map_err(|e| GrokError::Config(format!("Failed to create keyring entry: {e}")))?;

        entry
            .get_password()
            .map_err(|e| GrokError::Config(format!("API key not found in keyring: {e}")))
    }

    /// Delete an API key from secure storage
    pub fn delete_api_key(&self, provider: &str) -> Result<()> {
        let entry = Entry::new(&self.service, provider)
            .map_err(|e| GrokError::Config(format!("Failed to create keyring entry: {e}")))?;

        entry
            .delete_password()
            .map_err(|e| GrokError::Config(format!("Failed to delete API key: {e}")))?;

        Ok(())
    }

    /// Check if an API key exists in secure storage
    pub fn has_api_key(&self, provider: &str) -> bool {
        if let Ok(entry) = Entry::new(&self.service, provider) {
            entry.get_password().is_ok()
        } else {
            false
        }
    }
}

impl Default for KeyStore {
    fn default() -> Self {
        Self::new()
    }
}
