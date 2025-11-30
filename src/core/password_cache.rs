use std::collections::HashMap;

use secrecy::SecretString;
use secrecy::zeroize::Zeroize;

use super::keyring::{KeyringError, get_password};

/// Caches passwords from the keyring.
pub struct PasswordCache {
    cache: HashMap<String, SecretString>,
}

impl PasswordCache {
    /// Creates a new password cache.
    pub fn new() -> Self {
        PasswordCache {
            cache: HashMap::new(),
        }
    }

    /// Returns the password for `password_id`.
    /// If not cached, fetches from keyring and stores in the cache.
    pub fn get_password(&mut self, password_id: &str) -> Result<&SecretString, KeyringError> {
        if !self.cache.contains_key(password_id) {
            let password = get_password(password_id)?;
            self.cache.insert(password_id.to_string(), password);
        }

        Ok(self.cache.get(password_id).unwrap())
    }

    /// Explicitly clear all passwords when early cleanup is needed.
    #[allow(unused)]
    pub fn clear(&mut self) {
        for secret_string in self.cache.values_mut() {
            secret_string.zeroize(); // wipes memory
        }

        self.cache.clear();
    }
}

/// Impl Drop for PasswordCache.
impl Drop for PasswordCache {
    fn drop(&mut self) {
        // Ensures zeroing even on panic.
        self.clear();
    }
}

/// Impl Default for PasswordCache.
impl Default for PasswordCache {
    fn default() -> Self {
        Self::new()
    }
}
