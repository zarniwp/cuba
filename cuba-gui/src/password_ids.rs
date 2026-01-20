use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use cuba_lib::core::cuba::Cuba;

/// Defines a `PasswordIDs`.
pub struct PasswordIDs {
    ids: RwLock<HashSet<String>>,
    cuba: Arc<RwLock<Cuba>>,
}

/// Methods of `PasswordIDs`.
impl PasswordIDs {
    /// Creates a new `PasswordIDs`.
    pub fn new(cuba: Arc<RwLock<Cuba>>) -> Self {
        Self {
            ids: RwLock::new(HashSet::new()),
            cuba,
        }
    }

    /// Returns the password ids.
    pub fn get(&self) -> HashSet<String> {
        self.ids.read().unwrap().clone()
    }

    /// Refresh password ids from keyring.
    pub fn update(&self) {
        let mut ids = self.ids.write().unwrap();
        *ids = self
            .cuba
            .read()
            .unwrap()
            .get_password_ids()
            .unwrap_or_default();
    }
}
