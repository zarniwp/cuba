use std::{collections::HashMap, sync::Arc};

use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Serialize, Deserialize)]
pub struct LocalFS {
    /// Directory.
    pub dir: NPath<Abs, Dir>,
}

/// Defines a `WebDAVFS`.
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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
