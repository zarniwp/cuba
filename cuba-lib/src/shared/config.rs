use std::{collections::HashMap, fmt, sync::Arc};

use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use strum_macros::Display;

use crate::{send_error, send_info, shared::message::Message};

use super::npath::{Abs, Dir, NPath, Rel};

/// Load config from file.
pub fn load_config_from_file(sender: Sender<Arc<dyn Message>>, path: &str) -> Option<Config> {
    match std::fs::read_to_string(path) {
        Ok(config_str) => load_config_from_str(sender, &config_str),
        Err(err) => {
            send_error!(sender, err);
            None
        }
    }
}

/// Load config from &str.
pub fn load_config_from_str(sender: Sender<Arc<dyn Message>>, config: &str) -> Option<Config> {
    match toml::from_str::<Config>(config) {
        Ok(config) => Some(config),
        Err(err) => {
            send_error!(sender, err);
            None
        }
    }
}

/// Save config to file.
pub fn save_config_to_file(sender: Sender<Arc<dyn Message>>, path: &str, config: &Config) {
    let content = match toml::to_string_pretty(config) {
        Ok(content) => content,
        Err(err) => {
            send_error!(sender, err);
            return;
        }
    };

    match std::fs::write(path, content) {
        Ok(_) => send_info!(sender, "Config saved to {}", path),
        Err(err) => send_error!(sender, err),
    }
}

// Defines a `ConfigEntryType`.
#[derive(Display, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConfigEntryType {
    #[strum(to_string = "filesystem.local")]
    LocalFS,

    #[strum(to_string = "filesystem.webdav")]
    WebDAVFS,

    #[strum(to_string = "backup")]
    Backup,

    #[strum(to_string = "restore")]
    Restore,
}

/// Defines Methods for `ConfigEntryType`.
impl ConfigEntryType {
    /// Returns all `ConfigEntryType`s.
    pub const ALL: [Self; 4] = [Self::LocalFS, Self::WebDAVFS, Self::Backup, Self::Restore];
}

// Defines a `ConfigEntryKey`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigEntryKey {
    pub entry_type: ConfigEntryType,
    pub name: String,
}

/// Impl `Display` for `ConfigEntryKey`.
impl fmt::Display for ConfigEntryKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.\"{}\"", self.entry_type, self.name)
    }
}

// Defines a `ConfigEntryMut`.
pub enum ConfigEntryMut<'a> {
    LocalFS(&'a mut LocalFS),
    WebDAVFS(&'a mut WebDAVFS),
    Backup(&'a mut BackupConfig),
    Restore(&'a mut RestoreConfig),
}

/// Defines a `Config`.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Number of transfer threads.
    pub transfer_threads: usize,

    /// The filesystem profiles.
    pub filesystem: FilesystemConfig,

    /// The backup profiles.
    pub backup: HashMap<String, BackupConfig>,

    /// The restore profiles.
    pub restore: HashMap<String, RestoreConfig>,
}

/// Methods of `Config`.
impl Config {
    /// Checks if a password id is used in the config.
    pub fn has_password_id(&self, password_id: &str) -> bool {
        if self.filesystem.has_password_id(password_id) {
            return true;
        }

        for backup in self.backup.values() {
            if backup.has_password_id(password_id) {
                return true;
            }
        }

        false
    }

    /// Lists all entries in the config.
    pub fn list_entry_keys(&self) -> Vec<ConfigEntryKey> {
        let mut keys = Vec::new();

        for name in self.filesystem.local.keys() {
            keys.push(ConfigEntryKey {
                entry_type: ConfigEntryType::LocalFS,
                name: name.clone(),
            });
        }

        for name in self.filesystem.webdav.keys() {
            keys.push(ConfigEntryKey {
                entry_type: ConfigEntryType::WebDAVFS,
                name: name.clone(),
            });
        }

        for name in self.backup.keys() {
            keys.push(ConfigEntryKey {
                entry_type: ConfigEntryType::Backup,
                name: name.clone(),
            });
        }

        for name in self.restore.keys() {
            keys.push(ConfigEntryKey {
                entry_type: ConfigEntryType::Restore,
                name: name.clone(),
            });
        }

        keys
    }

    /// Lists all fs entries in the config.
    pub fn list_fs_keys(&self) -> Vec<ConfigEntryKey> {
        let mut keys = Vec::new();

        for name in self.filesystem.local.keys() {
            keys.push(ConfigEntryKey {
                entry_type: ConfigEntryType::LocalFS,
                name: name.clone(),
            });
        }

        for name in self.filesystem.webdav.keys() {
            keys.push(ConfigEntryKey {
                entry_type: ConfigEntryType::WebDAVFS,
                name: name.clone(),
            });
        }

        keys
    }

    /// Gets a mutable reference to a config entry.
    pub fn get_entry_mut(&mut self, key: &ConfigEntryKey) -> Option<ConfigEntryMut<'_>> {
        match key.entry_type {
            ConfigEntryType::LocalFS => self
                .filesystem
                .local
                .get_mut(&key.name)
                .map(ConfigEntryMut::LocalFS),

            ConfigEntryType::WebDAVFS => self
                .filesystem
                .webdav
                .get_mut(&key.name)
                .map(ConfigEntryMut::WebDAVFS),

            ConfigEntryType::Backup => self.backup.get_mut(&key.name).map(ConfigEntryMut::Backup),

            ConfigEntryType::Restore => {
                self.restore.get_mut(&key.name).map(ConfigEntryMut::Restore)
            }
        }
    }

    /// Adds a new entry to the config.
    pub fn add_new_entry(&mut self, entry_type: &ConfigEntryType, name: &str) {
        match entry_type {
            ConfigEntryType::LocalFS => {
                self.filesystem
                    .local
                    .insert(name.to_string(), LocalFS::default());
            }
            ConfigEntryType::WebDAVFS => {
                self.filesystem
                    .webdav
                    .insert(name.to_string(), WebDAVFS::default());
            }
            ConfigEntryType::Backup => {
                self.backup
                    .insert(name.to_string(), BackupConfig::default());
            }
            ConfigEntryType::Restore => {
                self.restore
                    .insert(name.to_string(), RestoreConfig::default());
            }
        }
    }

    /// Deletes the entry with the given key.
    pub fn delete_entry(&mut self, key: &ConfigEntryKey) {
        match key.entry_type {
            ConfigEntryType::LocalFS => {
                self.filesystem.local.remove(&key.name);
            }
            ConfigEntryType::WebDAVFS => {
                self.filesystem.webdav.remove(&key.name);
            }
            ConfigEntryType::Backup => {
                self.backup.remove(&key.name);
            }
            ConfigEntryType::Restore => {
                self.restore.remove(&key.name);
            }
        }
    }
}

/// Defines a `FilesystemConfig`.
#[derive(Debug, Serialize, Deserialize)]
pub struct FilesystemConfig {
    pub local: HashMap<String, LocalFS>,
    pub webdav: HashMap<String, WebDAVFS>,
}

/// Methods of `FilesystemConfig`.
impl FilesystemConfig {
    /// Checks if a password id is used in the filesystem config.
    pub fn has_password_id(&self, password_id: &str) -> bool {
        for webdav in self.webdav.values() {
            if webdav.password_id == password_id {
                return true;
            }
        }

        false
    }
}

// Defines a `LocalFS`.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LocalFS {
    /// Directory.
    pub dir: NPath<Abs, Dir>,
}

/// Defines a `WebDAVFS`.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WebDAVFS {
    /// Url.
    pub url: NPath<Abs, Dir>,

    /// Username.
    pub user: String,

    /// Password id.
    pub password_id: String,

    /// Connection timeout in seconds.
    pub timeout_secs: u64,
}

/// Defines a `BackupConfig`.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BackupConfig {
    /// The source filesystem.
    pub src_fs: String,

    /// The destination filesystem.
    pub dest_fs: String,

    /// The source directory.
    pub src_dir: NPath<Rel, Dir>,

    /// The destination directory.  
    pub dest_dir: NPath<Rel, Dir>,

    /// Optional inclusion patterns (glob).
    pub include: Option<Vec<String>>,

    /// Optional exclusion patterns (glob).
    pub exclude: Option<Vec<String>>,

    /// Encrypt?
    pub encrypt: bool,
    pub password_id: Option<String>,

    /// Compress?
    pub compression: bool,
}

/// Methods of `BackupConfig`.
impl BackupConfig {
    /// Checks if a password id is used in the backup config.
    pub fn has_password_id(&self, password_id: &str) -> bool {
        match &self.password_id {
            Some(id) => id == password_id,
            None => false,
        }
    }
}

/// Defines a `RestoreConfig`.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RestoreConfig {
    /// The source filesystem.
    pub src_fs: String,

    /// The destination filesystem.
    pub dest_fs: String,

    /// The source directory.
    pub src_dir: NPath<Rel, Dir>,

    /// The destination directory.  
    pub dest_dir: NPath<Rel, Dir>,

    /// Optional inclusion patterns (glob).
    pub include: Option<Vec<String>>,

    /// Optional exclusion patterns (glob).
    pub exclude: Option<Vec<String>>,
}

/// Example configuration file.
pub const EXAMPLE_CONFIG: &str = r#"
# Number of parallel threads to use for transfers
transfer_threads = 10

[filesystem.local."local_linux"]
# A local filesystem with base user
dir = "/home/user"

[filesystem.local."local_windows"]
# A local filesystem with base C
dir = "C:/"

[filesystem.webdav."remote_storage"]
# WebDAV server URL
url = "https://example.com/remote.php/dav/user"
# Username for authentication
user = "user"
# Identifier for password retrieval. Example: cuba password set webdav-pass
password_id = "webdav-pass"
# Connection timeout in seconds. Increase this, if the upload of large files
# failed due to timeout.
timeout_secs = 3600

[backup."backup_windows_documents"]
# Source and destination filesystems (must match keys from [filesystem])
src_fs = "local_windows"
dest_fs = "remote_storage"
src_dir = "user/Documents"
dest_dir = "backups/cuba"
# Optional inclusion patterns (glob)
include = ["**/*.txt"]
# Optional exclusion patterns (glob)
exclude = ["**/*.tmp"]
# Enable encryption
encrypt = true
# Optional password identifier for encryption
password_id = "backup-pass"
# Enable compression
compression = true

[restore."restore_windows_documents"]
# Source and destination filesystems (must match keys from [filesystem])
# Note: src/dest means source/destination of restore
src_fs = "remote_storage"
dest_fs = "local_windows"
src_dir = "backups/cuba"
dest_dir = "user/Documents/restored"
# Optional inclusion patterns (glob)
include = ["**/*.txt"]
# Optional exclusion patterns (glob)
exclude = ["**/*.tmp"]
"#;
