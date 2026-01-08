use keyring::Entry;
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;

/// Defines a `KeyringError`.
#[derive(Debug, Error)]
pub enum KeyringError {
    #[error("Entry creation error: {0}")]
    EntryCreation(String),

    #[error("Set password error: {0}")]
    SetPassword(String),

    #[error("Delete credential error: {0}")]
    DeleteCredential(String),

    #[error("Get password error: {0}")]
    GetPassword(String),
}

/// Helper to create a keyring entry
fn keyring_entry(id: &str) -> Result<Entry, KeyringError> {
    Entry::new("cuba", id).map_err(|err| KeyringError::EntryCreation(err.to_string()))
}

/// Store a password in OS keyring
pub fn store_password(id: &str, password: &SecretString) -> Result<(), KeyringError> {
    keyring_entry(id)?
        .set_password(password.expose_secret())
        .map_err(|err| KeyringError::SetPassword(err.to_string()))?;

    Ok(())
}

/// Remove a password from OS keyring
pub fn remove_password(id: &str) -> Result<(), KeyringError> {
    keyring_entry(id)?
        .delete_credential()
        .map_err(|err| KeyringError::DeleteCredential(err.to_string()))?;

    Ok(())
}

/// Retrieve a password and wrap in SecretString
pub fn get_password(id: &str) -> Result<SecretString, KeyringError> {
    let password = keyring_entry(id)?
        .get_password()
        .map_err(|err| KeyringError::GetPassword(err.to_string()))?;

    Ok(SecretString::new(password.into()))
}
