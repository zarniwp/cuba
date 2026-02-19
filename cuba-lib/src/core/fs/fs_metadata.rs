use std::time::SystemTime;

use crate::core::fs::fs_symlink_meta::FSSymlinkMeta;

/// Defines a `FSMetaData`
#[derive(Clone, Debug)]
pub struct FSMetaData {
    /// Creation time.
    pub created: Option<SystemTime>,

    /// Modified time.
    pub modified: Option<SystemTime>,

    /// Size in bytes.
    pub size: Option<u64>,

    // Symlink meta.
    pub symlink_meta: Option<FSSymlinkMeta>,
}

/// Methods for `FSMetaData`
impl FSMetaData {
    /// Creates a new `FSMetaData`.
    pub fn new(
        created: Option<SystemTime>,
        modified: Option<SystemTime>,
        size: Option<u64>,
        symlink_meta: Option<FSSymlinkMeta>,
    ) -> Self {
        Self {
            created,
            modified,
            size,
            symlink_meta,
        }
    }
}
