use std::{path::Path, sync::Arc};

use crossbeam_channel::Sender;
use toml_edit::{DocumentMut, Item};

use crate::{
    send_error, send_info,
    shared::{config::Config, message::Message},
};

/// Defines a `ConfigWriter`.
pub struct ConfigWriter;

/// Methods for `ConfigWriter`.
impl ConfigWriter {
    /// Writes a config file.
    pub fn write(sender: Sender<Arc<dyn Message>>, path: &Path, config: &Config) {
        let mut doc: DocumentMut;

        // Load existing config file if it exists.
        if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    doc = content
                        .parse::<DocumentMut>()
                        .unwrap_or_else(|_| DocumentMut::new())
                }
                Err(err) => {
                    send_error!(sender, err);
                    return;
                }
            }
        } else {
            doc = DocumentMut::new();
        }

        // Patch the config file.
        Self::patch_root(config, &mut doc);

        // Write back the config file.
        match std::fs::write(path, doc.to_string()) {
            Ok(_) => {
                send_info!(sender, "Config written to {}", path.display());
            }
            Err(err) => {
                send_error!(sender, err);
            }
        }
    }

    /// Patch the root table of the config file.
    fn patch_root(config: &Config, doc: &mut DocumentMut) {
        // transfer_threads
        doc["transfer_threads"] = toml_edit::value(config.transfer_threads as i64);

        // filesystem
        Self::patch_table(doc, "filesystem.local", &config.filesystem.local);
        Self::patch_table(doc, "filesystem.webdav", &config.filesystem.webdav);

        // backup
        Self::patch_table(doc, "backup", &config.backup);

        // restore
        Self::patch_table(doc, "restore", &config.restore);
    }

    /// Patch a table in the config file.
    fn patch_table<T: serde::Serialize>(
        doc: &mut DocumentMut,
        path: &str,
        map: &std::collections::HashMap<String, T>,
    ) {
        // Navigate to target table.
        let mut current = doc.as_table_mut();

        for part in path.split('.') {
            current = current
                .entry(part)
                .or_insert(Item::Table(Default::default()))
                .as_table_mut()
                .unwrap();
        }

        // Remove entries no longer present.
        let existing_keys: Vec<String> = current.iter().map(|(k, _)| k.to_string()).collect();

        for key in existing_keys {
            if !map.contains_key(&key) {
                current.remove(&key);
            }
        }

        // Insert / update entries.
        for (key, value) in map {
            let table = toml_edit::ser::to_document(value)
                .unwrap()
                .as_table()
                .clone();

            current[key] = Item::Table(table);
        }
    }
}
