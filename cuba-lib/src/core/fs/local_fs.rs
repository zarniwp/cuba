use crate::core::fs::fs_metadata::FSMetaData;
use crate::core::fs::fs_symlink_meta::{FSSymlinkMeta, FSSymlinkType};
use crate::shared::npath::{Abs, Dir, File, NPath, Symlink, UNPath};
use std::fs::FileType;
use std::io::{self, Read};
use std::path::Path;

use super::fs_base::FSBlockSize;
use super::fs_base::{FS, FSError, FSWrite};

/// Defines a `LocalFS`.
pub struct LocalFS {
    connected: bool,
}

/// Methods of `LocalFS`.
impl LocalFS {
    /// Creates a new `LocalFS`.
    pub fn new() -> Self {
        LocalFS { connected: false }
    }
}

impl Default for LocalFS {
    fn default() -> Self {
        Self::new()
    }
}

/// Impl of `FS` for `LocalFS`.
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

    fn meta(&self, abs_path: &UNPath<Abs>) -> Result<FSMetaData, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        let metadata = std::fs::symlink_metadata(abs_path.as_os_path())
            .map_err(|err| FSError::MetaFailed(abs_path.clone(), err.into()))?;

        // Target of metadata and abs_path must be the same.
        if metadata.file_type().is_dir() == abs_path.is_dir()
            || metadata.file_type().is_file() == abs_path.is_file()
            || metadata.file_type().is_symlink() == abs_path.is_symlink()
        {
            // Set metadata.
            let created = metadata.created().ok();
            let modified = metadata.modified().ok();
            let mut size = None;
            let mut symlink = None;

            // Is file?
            if metadata.is_file() {
                size = Some(metadata.len());
            }

            // Is symlink?
            if metadata.is_symlink() {
                let target_path = std::fs::read_link(abs_path.as_os_path())
                    .map_err(|err| FSError::MetaFailed(abs_path.clone(), err.into()))?;

                let target_type = symlink_type(&metadata.file_type());
                symlink = Some(FSSymlinkMeta::new(target_path, target_type));
            }

            let meta = FSMetaData::new(created, modified, size, symlink);

            Ok(meta)
        } else {
            Err(FSError::MetaFailed(
                abs_path.clone(),
                "Wrong path target".into(),
            ))
        }
    }

    fn list_dir(&self, abs_dir_path: &NPath<Abs, Dir>) -> Result<Vec<UNPath<Abs>>, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        let entries = std::fs::read_dir(abs_dir_path.as_os_path())
            .map_err(|err| FSError::ListDirFailed(abs_dir_path.clone(), err.into()))?;

        let mut paths = Vec::new();

        for entry in entries {
            let entry =
                entry.map_err(|err| FSError::ListDirFailed(abs_dir_path.clone(), err.into()))?;

            let metadata = std::fs::symlink_metadata(entry.path())
                .map_err(|err| FSError::ListDirFailed(abs_dir_path.clone(), err.into()))?;

            match entry.path().to_str() {
                Some(entry_str) => {
                    // Only process files and directories, skip symlinks and others.
                    if metadata.file_type().is_file() {
                        let entry_abs_path =
                            UNPath::File(NPath::<Abs, File>::try_from(entry_str).map_err(
                                |err| FSError::ListDirFailed(abs_dir_path.clone(), err.into()),
                            )?);

                        paths.push(entry_abs_path);
                    } else if metadata.file_type().is_dir() {
                        let entry_abs_path =
                            UNPath::Dir(NPath::<Abs, Dir>::try_from(entry_str).map_err(|err| {
                                FSError::ListDirFailed(abs_dir_path.clone(), err.into())
                            })?);

                        paths.push(entry_abs_path);
                    } else if metadata.file_type().is_symlink() {
                        let entry_abs_path =
                            UNPath::Symlink(NPath::<Abs, Symlink>::try_from(entry_str).map_err(
                                |err| FSError::ListDirFailed(abs_dir_path.clone(), err.into()),
                            )?);

                        paths.push(entry_abs_path);
                    } else {
                        return Err(FSError::ListDirFailed(
                            abs_dir_path.clone(),
                            "Unkown file type".into(),
                        ));
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

        Ok(paths)
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

    fn mklink(
        &self,
        abs_sym_path: &NPath<Abs, Symlink>,
        symlink_meta: &FSSymlinkMeta,
    ) -> Result<(), FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        match create_symlink(
            &abs_sym_path.as_os_path(),
            &symlink_meta.target_path,
            &symlink_meta.target_type,
        ) {
            Ok(_) => Ok(()),
            Err(err) => Err(FSError::MkLinkFailed(abs_sym_path.clone(), err.into())),
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

/// Returns a symlink type.
fn symlink_type(file_type: &FileType) -> FSSymlinkType {
    #[cfg(windows)]
    {
        windows::symlink_type(file_type)
    }

    #[cfg(unix)]
    {
        unix::symlink_type(file_type)
    }
}

/// Creates a symlink.
fn create_symlink(
    link_path: &Path,
    target_path: &Path,
    target_type: &FSSymlinkType,
) -> io::Result<()> {
    #[cfg(windows)]
    {
        windows::create_symlink(link_path, target_path, target_type)
    }

    #[cfg(unix)]
    {
        unix::create_symlink(link_path, target_path)
    }
}

#[cfg(unix)]
mod unix {
    use crate::core::fs::fs_symlink_meta::FSSymlinkType;
    use std::fs::FileType;
    use std::io;
    use std::path::Path;

    /// Returns a symlink type.
    pub fn symlink_type(_file_type: &FileType) -> FSSymlinkType {
        FSSymlinkType::Unknown
    }

    /// Creates a symlink.
    pub fn create_symlink(link_path: &Path, target_path: &Path) -> io::Result<()> {
        std::os::unix::fs::symlink(target_path, link_path)
    }
}

/// Config for Windows
#[cfg(windows)]
mod windows {
    use crate::core::fs::fs_symlink_meta::FSSymlinkType;
    use std::fs::FileType;
    use std::io;
    use std::os::windows::fs::FileTypeExt;
    use std::path::Path;

    /// Returns the symlink type.
    pub fn symlink_type(file_type: &FileType) -> FSSymlinkType {
        if file_type.is_symlink_file() {
            FSSymlinkType::File
        } else if file_type.is_symlink_dir() {
            FSSymlinkType::Dir
        } else {
            FSSymlinkType::Unknown
        }
    }

    /// Creates a symlink.
    pub fn create_symlink(
        link_path: &Path,
        target_path: &Path,
        target_type: &FSSymlinkType,
    ) -> io::Result<()> {
        match target_type {
            FSSymlinkType::File => std::os::windows::fs::symlink_file(target_path, link_path),
            FSSymlinkType::Dir => std::os::windows::fs::symlink_dir(target_path, link_path),
            FSSymlinkType::Unknown => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid symlink type",
            )),
        }
    }
}
