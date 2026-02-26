use std::collections::HashSet;

use keyring::Entry;
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;

/// The user of the password ids.
/// It contains a list of all ids.
const USER_PASSWORD_IDS: &str = "password-ids";

/// Defines a `KeyringError`.
#[derive(Debug, Error)]
pub enum KeyringError {
    #[error("Entry creation error: {0}")]
    EntryCreation(String),

    #[error("Store credential error: {0}")]
    StoreCredential(String),

    #[error("Delete credential error: {0}")]
    DeleteCredential(String),

    #[error("Retrieve credential error: {0}")]
    RetrieveCredential(String),

    #[error("ID contains invalid characters or has an invalid length")]
    PasswordIDInvalid,

    #[error("Serialization error: {0}")]
    Serialize(String),

    #[error("Deserialization error: {0}")]
    Deserialize(String),
}

/// Defines `Operation`s for the `update_password_id`.
#[derive(Debug, Copy, Clone)]
enum Operation {
    Insert,
    Remove,
}

/// Is valid id?
fn is_valid_id(id: &str) -> bool {
    if id == USER_PASSWORD_IDS {
        false
    } else {
        let len = id.len();
        (1..=64).contains(&len)
            && id.bytes().all(|byte| {
                matches!(byte,
                    b'A'..=b'Z'
                    | b'a'..=b'z'
                    | b'0'..=b'9'
                    | b'_'
                    | b'-'
                    | b'.'
                )
            })
    }
}

/// Helper to create a keyring entry.
fn keyring_entry(id: &str) -> Result<Entry, KeyringError> {
    Entry::new("cuba", id).map_err(|err| KeyringError::EntryCreation(err.to_string()))
}

/// Helper to update the password ids.
fn update_password_ids(id: &str, operation: Operation) -> Result<(), KeyringError> {
    let entry_ids = keyring_entry(USER_PASSWORD_IDS)?;

    let mut set_ids: HashSet<String> = match entry_ids.get_secret() {
        Ok(bytes_ids) => wincode::deserialize(&bytes_ids)
            .map_err(|err| KeyringError::Deserialize(err.to_string()))?,
        Err(_) => HashSet::new(),
    };

    // Change the set of ids.
    match operation {
        Operation::Insert => {
            set_ids.insert(id.to_string());
        }
        Operation::Remove => {
            set_ids.remove(id);
        }
    }

    let bytes_ids =
        wincode::serialize(&set_ids).map_err(|err| KeyringError::Serialize(err.to_string()))?;

    entry_ids
        .set_secret(&bytes_ids)
        .map_err(|err| KeyringError::StoreCredential(err.to_string()))?;

    Ok(())
}

/// Store a password in OS keyring.
pub fn store_password(id: &str, password: &SecretString) -> Result<(), KeyringError> {
    if !is_valid_id(id) {
        return Err(KeyringError::PasswordIDInvalid);
    }

    // Only update password_ids if set password was successful.
    keyring_entry(id)?
        .set_password(password.expose_secret())
        .map_err(|err| KeyringError::StoreCredential(err.to_string()))?;

    update_password_ids(id, Operation::Insert)?;
    Ok(())
}

/// Removes a password from OS keyring
pub fn remove_password(id: &str) -> Result<(), KeyringError> {
    // Prevent deletion of USER_PASSWORD_IDS.
    if id == USER_PASSWORD_IDS {
        return Err(KeyringError::DeleteCredential(
            "Cannot delete password-ids".to_string(),
        ));
    }

    // Only update password_ids if remove password was successful.
    keyring_entry(id)?
        .delete_credential()
        .map_err(|err| KeyringError::DeleteCredential(err.to_string()))?;

    update_password_ids(id, Operation::Remove)?;
    Ok(())
}

/// Retrieve a password and wrap in SecretString.
pub fn get_password(id: &str) -> Result<SecretString, KeyringError> {
    let password = keyring_entry(id)?
        .get_password()
        .map_err(|err| KeyringError::RetrieveCredential(err.to_string()))?;

    Ok(SecretString::new(password.into()))
}

// Returns the list of stored password ids.
pub fn get_password_ids() -> Result<HashSet<String>, KeyringError> {
    let entry = keyring_entry(USER_PASSWORD_IDS)?;

    let set = match entry.get_secret() {
        Ok(bytes) => wincode::deserialize(&bytes)
            .map_err(|err| KeyringError::Deserialize(err.to_string()))?,
        Err(_) => HashSet::new(),
    };

    Ok(set)
}
