use std::io::{Read, Write};
use std::sync::{Arc, RwLock};

use crate::core::fs::fs_metadata::FSMetaData;
use crate::core::fs::fs_symlink_meta::FSSymlinkMeta;
use crate::shared::npath::{Abs, Dir, File, NPath, Symlink, UNPath};

use super::fs_base::FSBlockSize;
use super::fs_base::{FS, FSError, FSMount, FSWrite};

/// Methods of `FSMount`.
impl FSMount {
    /// Creates dev_null filesystem mount.
    pub fn dev_null() -> Self {
        FSMount {
            fs: Arc::new(RwLock::new(NullFS::new())),
            abs_dir_path: Arc::new(NPath::default()),
        }
    }
}

struct DevNull;

/// Impl of `Read` for `DevNull`.
impl Write for DevNull {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len()) // Pretend we "wrote" everything.
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Defines a null filesystem.
///
/// A struct representing a null fs that implements the FS trait.
pub struct NullFS {
    connected: bool,
}

/// Methods of `NullFS`.
impl NullFS {
    /// Creates a new `NullFS`.
    pub fn new() -> Self {
        NullFS { connected: false }
    }
}

/// Impl of `Default` for `NullFS`.
impl Default for NullFS {
    fn default() -> Self {
        Self::new()
    }
}

/// Impl of `FS` for `NullFS`.
impl FS for NullFS {
    fn connect(&mut self) -> Result<(), FSError> {
        // Set connection state to true.
        self.connected = true;
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), FSError> {
        // Set connection state to false.
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn block_size(&self) -> FSBlockSize {
        FSBlockSize::new(None, 4096, None)
    }

    fn meta(&self, _abs_path: &UNPath<Abs>) -> Result<FSMetaData, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        Err(FSError::NotConnected)
    }

    fn list_dir(&self, _abs_dir_path: &NPath<Abs, Dir>) -> Result<Vec<UNPath<Abs>>, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        Err(FSError::NotSupported)
    }

    fn remove_file(&self, _abs_file_path: &NPath<Abs, File>) -> Result<(), FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        Err(FSError::NotSupported)
    }

    fn remove_dir(&self, _abs_dir_path: &NPath<Abs, Dir>) -> Result<(), FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        Err(FSError::NotSupported)
    }

    fn mkdir(&self, _abs_dir_path: &NPath<Abs, Dir>) -> Result<(), FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        Err(FSError::NotSupported)
    }

    fn mklink(
        &self,
        _abs_sym_path: &NPath<Abs, Symlink>,
        _symlink_meta: &FSSymlinkMeta,
    ) -> Result<(), FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        Err(FSError::NotSupported)
    }

    fn read_data(
        &self,
        _abs_file_path: &NPath<Abs, File>,
    ) -> Result<Box<dyn Read + Send>, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        Err(FSError::NotSupported)
    }

    fn write_data(&self, _abs_file_path: &NPath<Abs, File>) -> Result<FSWrite, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        Ok(FSWrite::new(Box::new(DevNull), None))
    }
}
