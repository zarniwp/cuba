use std::io::Read;
use warned::Warned;

use crate::shared::npath::{Abs, Dir, File, NPath, UNPath};

use super::fs_base::FSBlockSize;
use super::fs_base::{FS, FSError, FSWrite};
use super::fs_node::{FSNode, FSNodeMetaData};

/// A struct representing a local fs that implements the FS trait.
pub struct LocalFS {
    connected: bool,
}

impl LocalFS {
    /// Creates a new instance of `LocalFS`.
    pub fn new() -> Self {
        LocalFS { connected: false }
    }
}

impl Default for LocalFS {
    fn default() -> Self {
        Self::new()
    }
}

impl FS for LocalFS {
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

    fn meta(&self, abs_path: &UNPath<Abs>) -> Result<FSNodeMetaData, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        let metadata = std::fs::metadata(abs_path.as_os_path())
            .map_err(|err| FSError::MetaFailed(abs_path.clone(), err.into()))?;

        // Target of metadata and abs_path must be the same.
        if metadata.file_type().is_dir() == abs_path.is_dir()
            || metadata.file_type().is_file() == abs_path.is_file()
        {
            // Set metadata.
            let created = metadata.created().ok().unwrap();
            let modified = metadata.modified().ok().unwrap();
            let size = metadata.len();

            let meta = FSNodeMetaData {
                created,
                modified,
                size,
            };

            Ok(meta)
        } else {
            Err(FSError::MetaFailed(
                abs_path.clone(),
                "Wrong path target".into(),
            ))
        }
    }

    fn list_dir(
        &self,
        abs_dir_path: &NPath<Abs, Dir>,
    ) -> Result<Warned<Vec<FSNode>, String>, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        let entries = std::fs::read_dir(abs_dir_path.as_os_path())
            .map_err(|err| FSError::ListDirFailed(abs_dir_path.clone(), err.into()))?;

        let mut fs_nodes = Vec::new();
        let mut warnings = Vec::<String>::new();

        for entry in entries {
            let entry =
                entry.map_err(|err| FSError::ListDirFailed(abs_dir_path.clone(), err.into()))?;
            let entry_path = entry.path();
            let metadata = std::fs::metadata(&entry_path)
                .map_err(|err| FSError::ListDirFailed(abs_dir_path.clone(), err.into()))?;

            let created = metadata.created().ok().unwrap();
            let modified = metadata.modified().ok().unwrap();
            let size = metadata.len();

            let fs_metadata = FSNodeMetaData {
                created,
                modified,
                size,
            };

            match entry_path.to_str() {
                Some(entry_str) => {
                    // Only process files and directories, skip symlinks and others.
                    if metadata.file_type().is_file() {
                        let entry_abs_path =
                            UNPath::File(NPath::<Abs, File>::try_from(entry_str).map_err(
                                |err| FSError::ListDirFailed(abs_dir_path.clone(), err.into()),
                            )?);

                        fs_nodes.push(FSNode {
                            abs_path: entry_abs_path,
                            metadata: fs_metadata,
                        });
                    } else if metadata.file_type().is_dir() {
                        let entry_abs_path =
                            UNPath::Dir(NPath::<Abs, Dir>::try_from(entry_str).map_err(|err| {
                                FSError::ListDirFailed(abs_dir_path.clone(), err.into())
                            })?);

                        fs_nodes.push(FSNode {
                            abs_path: entry_abs_path,
                            metadata: fs_metadata,
                        });
                    } else {
                        warnings.push(format!("{} is a symlink and ignored", entry_str));
                        // Do not push a node for symlinks or unsupported types.
                    }
                }
                None => {
                    return Err(FSError::ListDirFailed(
                        abs_dir_path.clone(),
                        "Path is not in valid unicode".into(),
                    ));
                }
            }
        }

        Ok(Warned::new(fs_nodes, warnings))
    }

    fn remove_file(&self, abs_file_path: &NPath<Abs, File>) -> Result<(), FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        match std::fs::remove_file(abs_file_path.as_os_path()) {
            Ok(_) => Ok(()),
            Err(err) => Err(FSError::RemoveFileFailed(abs_file_path.clone(), err.into())),
        }
    }

    fn remove_dir(&self, abs_dir_path: &NPath<Abs, Dir>) -> Result<(), FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        match std::fs::remove_dir(abs_dir_path.as_os_path()) {
            Ok(_) => Ok(()),
            Err(err) => Err(FSError::RemoveDirFailed(abs_dir_path.clone(), err.into())),
        }
    }

    fn mkdir(&self, abs_dir_path: &NPath<Abs, Dir>) -> Result<(), FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        match std::fs::create_dir(abs_dir_path.as_os_path()) {
            Ok(_) => Ok(()),
            Err(err) => Err(FSError::MkDirFailed(abs_dir_path.clone(), err.into())),
        }
    }

    fn read_data(&self, abs_file_path: &NPath<Abs, File>) -> Result<Box<dyn Read + Send>, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        // Attempt to open the file.
        let file = std::fs::File::open(abs_file_path.as_os_path())
            .map_err(|err| FSError::ReadFailed(abs_file_path.clone(), err.into()))?;

        // Return the file as a `Box<dyn Read>`.
        Ok(Box::new(file)) // This is where the `Box<dyn Read>` comes in.
    }

    fn write_data(&self, abs_file_path: &NPath<Abs, File>) -> Result<FSWrite, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        // Attempt to open the file in write mode (create if doesn't exist).
        let file = std::fs::File::create(abs_file_path.as_os_path())
            .map_err(|err| FSError::WriteFailed(abs_file_path.clone(), err.into()))?;

        // Return the file wrapped in a `Box<dyn Write>`.
        Ok(FSWrite::new(Box::new(file), None)) // This is where the `Box<dyn Write>` comes in.
    }
}
