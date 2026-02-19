use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Defines a `FSSymlinkType`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FSSymlinkType {
    File,
    Dir,
    Unknown,
}

/// Defines a `FSSymlinkMeta`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FSSymlinkMeta {
    /// Target path.
    pub target_path: PathBuf,

    /// Target type.
    pub target_type: FSSymlinkType,
}

/// Methods for `FSSymlinkMeta`.
impl FSSymlinkMeta {
    /// Creates a new `FSSymlinkMeta`.
    pub fn new(target_path: PathBuf, target_type: FSSymlinkType) -> Self {
        Self {
            target_path,
            target_type,
        }
    }
}
