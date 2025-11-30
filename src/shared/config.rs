use std::collections::HashMap;

use serde::Deserialize;

use super::npath::{Abs, Dir, NPath, Rel};

#[derive(Debug, Deserialize)]
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

impl Config {
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

/// The filesystem profiles.
#[derive(Debug, Deserialize)]
pub struct FilesystemConfig {
    pub local: HashMap<String, LocalFS>,
    pub webdav: HashMap<String, WebDAVFS>,
}

impl FilesystemConfig {
    pub fn has_password_id(&self, password_id: &str) -> bool {
        for webdav in self.webdav.values() {
            if webdav.password_id == password_id {
                return true;
            }
        }

        false
    }
}

/// The local filesystem profiles.
#[derive(Debug, Deserialize)]
pub struct LocalFS {
    /// Directory.
    pub dir: NPath<Abs, Dir>,
}

/// The WebDAV filesystem profiles.
#[derive(Debug, Deserialize)]
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

/// The backup profiles.
#[derive(Debug, Deserialize)]
pub struct BackupConfig {
    /// The source filesystem.
    pub src_fs: String,

    /// The destination filesystem.
    pub dest_fs: String,

    /// The source directory.
    pub src_dir: NPath<Rel, Dir>,

    /// The destination directory.  
    pub dest_dir: NPath<Rel, Dir>,

    /// Optional inclusion patterns (glob)
    pub include: Option<Vec<String>>,

    /// Optional exclusion patterns (glob)
    pub exclude: Option<Vec<String>>,

    /// Encrypt?
    pub encrypt: bool,
    pub password_id: Option<String>,

    /// Compress?
    pub compression: bool,
}

impl BackupConfig {
    pub fn has_password_id(&self, password_id: &str) -> bool {
        match &self.password_id {
            Some(id) => id == password_id,
            None => false,
        }
    }
}

/// The restore profiles.
#[derive(Debug, Deserialize)]
pub struct RestoreConfig {
    /// The source filesystem.
    pub src_fs: String,

    /// The destination filesystem.
    pub dest_fs: String,

    /// The source directory.
    pub src_dir: NPath<Rel, Dir>,

    /// The destination directory.  
    pub dest_dir: NPath<Rel, Dir>,

    /// Optional inclusion patterns (glob)
    pub include: Option<Vec<String>>,

    /// Optional exclusion patterns (glob)
    pub exclude: Option<Vec<String>>,
}

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
