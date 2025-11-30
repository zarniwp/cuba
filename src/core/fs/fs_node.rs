use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::shared::npath::{Abs, UNPath};

/// `FSNode` metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FSNodeMetaData {
    /// Creation time.
    pub created: SystemTime,

    /// Modify time.
    pub modified: SystemTime,

    /// Size in bytes.
    pub size: u64,
}

/// `FSNode` with path and metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FSNode {
    /// FSNode path.
    pub abs_path: UNPath<Abs>,

    /// FSNode metadata.
    pub metadata: FSNodeMetaData,
}

impl FSNode {
    #![allow(unused)]
    // Returns true, if `FSNode` is directory.
    pub fn is_dir(&self) -> bool {
        self.abs_path.is_dir()
    }
}
