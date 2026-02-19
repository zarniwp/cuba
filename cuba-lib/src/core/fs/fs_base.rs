use std::error::Error;
use std::io::{Read, Write};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use thiserror::Error;

use crate::core::fs::fs_metadata::FSMetaData;
use crate::shared::npath::{Abs, Dir, File, NPath, UNPath};

pub type FSHandle = Arc<RwLock<dyn FS>>;

/// A mount point of a filesystem.
pub struct FSMount {
    pub fs: FSHandle,
    pub abs_dir_path: Arc<NPath<Abs, Dir>>,
}

impl FSMount {
    /// Creates a new `FSMount`.
    pub fn new(fs: FSHandle, abs_dir_path: Arc<NPath<Abs, Dir>>) -> Self {
        FSMount { fs, abs_dir_path }
    }
}

impl Clone for FSMount {
    /// Clone the FSMount, shares the same FS handle and path.
    fn clone(&self) -> Self {
        Self {
            fs: Arc::clone(&self.fs),
            abs_dir_path: Arc::clone(&self.abs_dir_path),
        }
    }
}

/// Defines a connection of two file systems.
pub struct FSConnection {
    pub src_mnt: FSMount,
    pub dest_mnt: FSMount,
}

/// Methods of `FSConnection`.
impl FSConnection {
    /// Creates a new `FSConnection`.
    pub fn new(src_mnt: FSMount, dest_mnt: FSMount) -> Self {
        FSConnection { src_mnt, dest_mnt }
    }

    /// Opens the connection. This means to ensure both file systems are connected.
    pub fn open(&self) -> Result<(), FSError> {
        if !self.src_mnt.fs.read().unwrap().is_connected() {
            self.src_mnt.fs.write().unwrap().connect()?;
        }

        if !self.dest_mnt.fs.read().unwrap().is_connected() {
            self.dest_mnt.fs.write().unwrap().connect()?;
        }

        Ok(())
    }

    /// Closes the connection. This means to ensure both file systems are disconnected.
    pub fn close(&self) -> Result<(), FSError> {
        if self.src_mnt.fs.read().unwrap().is_connected() {
            self.src_mnt.fs.write().unwrap().disconnect()?;
        }

        if self.dest_mnt.fs.read().unwrap().is_connected() {
            self.dest_mnt.fs.write().unwrap().disconnect()?;
        }

        Ok(())
    }
}

/// Impl of `Clone` for `FSConnection`.
impl Clone for FSConnection {
    /// Clone the FSConnection, shares the FSMounts.
    fn clone(&self) -> Self {
        Self {
            src_mnt: self.src_mnt.clone(),
            dest_mnt: self.dest_mnt.clone(),
        }
    }
}

/// The block size represents the minimum, recommended and maximum number of bytes that can be read
/// at once by [`read_data`] or written at once by [`write_data`]. This value helps
/// optimize I/O operations by aligning reads and writes to efficient chunk sizes.
pub struct FSBlockSize {
    pub minimum: Option<usize>,
    pub recommended: usize,
    pub maximum: Option<usize>,
}

/// Methods of `FSBlockSize`.
impl FSBlockSize {
    /// Creates a new `FSBlockSize`.
    pub fn new(minimum: Option<usize>, recommended: usize, maximum: Option<usize>) -> Self {
        FSBlockSize {
            minimum,
            recommended,
            maximum,
        }
    }

    /// Chooses the optimal block size between two `FSBlockSize` values.
    pub fn choose(block_size_1: &FSBlockSize, block_size_2: &FSBlockSize) -> usize {
        // Take the maximum of the two minimums (if any).
        let min = match (block_size_1.minimum, block_size_2.minimum) {
            (Some(min_1), Some(min_2)) => Some(min_1.max(min_2)),
            (Some(min), None) | (None, Some(min)) => Some(min),
            (None, None) => None,
        };

        // Take the minimum of the two maximums (if any).
        let max = match (block_size_1.maximum, block_size_2.maximum) {
            (Some(max_1), Some(max_2)) => Some(max_1.min(max_2)),
            (Some(max), None) | (None, Some(max)) => Some(max),
            (None, None) => None,
        };

        // Try to use the higher of the two recommended sizes.
        let mut chosen = block_size_1.recommended.max(block_size_2.recommended);

        // Clamp between min and max if needed.
        if let Some(min_value) = min
            && chosen < min_value
        {
            chosen = min_value;
        }
        if let Some(max_value) = max
            && chosen > max_value
        {
            chosen = max_value;
        }

        chosen
    }
}

/// Defines a writer for the fs.
pub struct FSWrite {
    writer: Option<Box<dyn Write + Send>>,
    thread_handle: Option<JoinHandle<()>>,
}

impl FSWrite {
    /// Creates a new `FSWrite`.
    pub fn new(writer: Box<dyn Write + Send>, thread_handle: Option<JoinHandle<()>>) -> Self {
        FSWrite {
            writer: Some(writer),
            thread_handle,
        }
    }

    /// Finishes the `FSWrite`.
    pub fn finish(mut self) {
        // Close the write side
        if let Some(mut writer) = self.writer.take() {
            let _ = writer.flush();
            // Dropping happens here when it goes out of scope
        }

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Impl for `Write`.
impl Write for FSWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(writer) = self.writer.as_mut() {
            writer.write(buf)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "FSWrite closed",
            ))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(writer) = self.writer.as_mut() {
            writer.flush()
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "FSWrite closed",
            ))
        }
    }
}

/// Drops the `FSWrite`.
impl Drop for FSWrite {
    fn drop(&mut self) {
        // Close the write side
        if let Some(mut writer) = self.writer.take() {
            let _ = writer.flush();
            // Dropping happens here when it goes out of scope
        }

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Defines a custom error type for the file system (FS).
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum FSError {
    /// Error when the fs fails to establish a connection.
    #[error("Failed to connect {0}")]
    ConnectionFailed(#[source] Box<dyn Error + Send + Sync>),

    /// Error when the fs is not connected.
    #[error("No connection established.")]
    NotConnected,

    /// Error when the operation is not supported.
    #[error("Operation not supported.")]
    NotSupported,

    /// Error when the metadata of the file, directory or symlink cannot be retreived, including the destination path.
    #[error("Failed to retrieve meta data {0:?}")]
    MetaFailed(UNPath<Abs>, #[source] Box<dyn Error + Send + Sync>),

    /// Error when the destination cannot be listed, including the destination path.
    #[error("Failed to list directory {0:?}")]
    ListDirFailed(NPath<Abs, Dir>, #[source] Box<dyn Error + Send + Sync>),

    /// Error when a file  cannot be removed, including the destination path.
    #[error("Failed to remove file {0:?}")]
    RemoveFileFailed(NPath<Abs, File>, #[source] Box<dyn Error + Send + Sync>),

    /// Error when a directory cannot be removed, including the destination path.
    #[error("Failed to remove directory {0:?}")]
    RemoveDirFailed(NPath<Abs, Dir>, #[source] Box<dyn Error + Send + Sync>),

    /// Error when a directory cannot be created, including the destination dir path.
    #[error("Failed to create directory {0:?}")]
    MkDirFailed(NPath<Abs, Dir>, #[source] Box<dyn Error + Send + Sync>),

    /// Error when reading data from a file fails, including the source file path.
    #[error("Failed to read data from file {0:?}")]
    ReadFailed(NPath<Abs, File>, #[source] Box<dyn Error + Send + Sync>),

    /// Error when writing data to a file fails, including the destination file path.
    #[error("Failed to write data to file {0:?}")]
    WriteFailed(NPath<Abs, File>, #[source] Box<dyn Error + Send + Sync>),
}

/// Defines the interface (trait) that a fs must implement.
#[allow(dead_code)]
pub trait FS: Send + Sync {
    /// Establishes a connection for the fs.
    ///
    /// # Errors
    ///
    /// Returns [`FSError::ConnectionFailed`] when the connection can't be established or a connection already exists.
    fn connect(&mut self) -> Result<(), FSError>;

    /// Disconnects the fs.
    ///
    /// # Errors
    ///
    /// Returns [`FSError::NotConnected`] when the fs is not connected.
    fn disconnect(&mut self) -> Result<(), FSError>;

    /// Returns `true` if the fs is currently connected; otherwise returns `false`.
    fn is_connected(&self) -> bool;

    /// Returns the block size in bytes of the filesystem.
    ///
    /// The block size represents the minimum, recommended and maximum number of bytes that can be read
    /// at once by [`read_data`] or written at once by [`write_data`]. This value helps
    /// optimize I/O operations by aligning reads and writes to efficient chunk sizes.
    ///
    /// # Returns
    ///
    /// The block size in bytes.
    fn block_size(&self) -> FSBlockSize;

    /// Returns metadata of the file or directory at the specified `abs_path`.
    /// Returns and error, when the resource does not exist or the resource
    /// has not the same target (file, dir) as the UNPath.
    ///
    /// # Errors
    ///
    /// - Returns [`FSError::NotConnected`] when the fs is not connected.
    /// - Returns [`FSError::MetaFailed`] when `meta` failes.
    fn meta(&self, abs_path: &UNPath<Abs>) -> Result<FSMetaData, FSError>;

    /// List directory entries at the specified `abs_dir_path`.
    ///
    /// # Errors
    ///
    /// - Returns [`FSError::NotConnected`] when the fs is not connected.
    /// - Returns [`FSError::ListDirFailed`] when `list_dir` failes.
    fn list_dir(&self, abs_dir_path: &NPath<Abs, Dir>) -> Result<Vec<UNPath<Abs>>, FSError>;

    /// Walks through a directory recursively and executes a callback function on each entry.
    ///
    /// This function traverses a directory and all its subdirectories, invoking `callback`
    /// on each file and directory encountered.
    ///
    /// # Arguments
    ///
    /// - `abs_dir_path` - The root path where traversal starts.
    /// - `callback` - A function that will be executed for each encountered file or directory.
    ///
    /// If callback returns true on a directory, walk continues traversing the directory.
    /// `error_callback` - A function that will be executed for each encountered error.
    ///
    /// # Errors
    ///
    /// - Returns [`FSError::NotConnected`] when the fs is not connected.
    fn walk_dir_rec(
        &self,
        abs_dir_path: &NPath<Abs, Dir>,
        callback: &mut dyn FnMut(UNPath<Abs>) -> bool,
        error_callback: &dyn Fn(FSError),
    ) -> Result<(), FSError> {
        if !self.is_connected() {
            return Err(FSError::NotConnected);
        }

        match self.list_dir(abs_dir_path) {
            Ok(entries) => {
                for abs_path in entries {
                    match &abs_path {
                        UNPath::File(_abs_file_path) => {
                            callback(abs_path);
                        }
                        UNPath::Dir(abs_dir_path) => {
                            if callback(abs_path.clone()) {
                                self.walk_dir_rec(abs_dir_path, callback, error_callback)?
                            }
                        }
                        UNPath::Symlink(_abs_sym_path) => {
                            callback(abs_path);
                        }
                    }
                }
            }
            Err(err) => {
                error_callback(err);
            }
        }

        Ok(())
    }

    /// Removes the file at the specified `abs_file_path`.
    ///
    /// # Errors
    ///
    /// - Returns [`FSError::NotConnected`] when the fs is not connected.
    /// - Returns [`FSError::RemoveFailed`] when `remove_file` failed.
    fn remove_file(&self, abs_file_path: &NPath<Abs, File>) -> Result<(), FSError>;

    /// Removes the directory at the specified `abs_dir_path`.
    ///
    /// # Errors
    ///
    /// - Returns [`FSError::NotConnected`] when the fs is not connected.
    /// - Returns [`FSError::RemoveFailed`] when `remove_dir` failed.
    fn remove_dir(&self, abs_dir_path: &NPath<Abs, Dir>) -> Result<(), FSError>;

    /// Creates a directory at the specified `abs_dir_path`.
    ///
    /// # Errors
    ///
    /// - Returns [`FSError::NotConnected`] when the fs is not connected.
    /// - Returns [`FSError::MkDirFailed`] when `mkdir` failed.
    fn mkdir(&self, abs_dir_path: &NPath<Abs, Dir>) -> Result<(), FSError>;

    /// Reads binary data from the file `abs_file_path`.
    /// Returns a reader.
    ///
    /// # Errors
    ///
    /// - Returns [`FSError::NotConnected`] when the fs is not connected.
    /// - Returns [`FSError::ReadFailed`] when `read_data` failed.
    fn read_data(&self, abs_file_path: &NPath<Abs, File>) -> Result<Box<dyn Read + Send>, FSError>;

    /// Writes binary data to the file `abs_file_path`.
    /// Returns a `FSWrite`.
    ///
    /// # Errors
    ///
    /// - Returns [`FSError::NotConnected`] when the fs is not connected.
    /// - Returns [`FSError::WriteFailed`] when `write_data` failed.
    fn write_data(&self, abs_file_path: &NPath<Abs, File>) -> Result<FSWrite, FSError>;
}
