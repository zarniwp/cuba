#![allow(unused)]

use crossbeam_channel::Sender;
use secrecy::SecretString;
use std::sync::{Arc, RwLock};

use crate::send_error;
use crate::send_info;
use crate::shared::{
    config::Config,
    message::{Message, StringError},
    npath::{Dir, NPath, Rel},
};

use super::backup::run_backup;
use super::clean::run_clean;
use super::fs::{
    fs_base::{FSConnection, FSMount},
    local_fs::LocalFS,
    webdav_fs::WebDAVFS,
};
use super::keyring::{get_password, remove_password, store_password};
use super::restore::run_restore;
use super::verify::run_verify;

/// Creates a filesystem mount from the config.
fn create_fs_mount(
    config: &Config,
    fs: &str,
    rel_dir_path: &NPath<Rel, Dir>,
) -> Result<FSMount, Arc<dyn std::error::Error + Send + Sync + 'static>> {
    if let Some(local_fs) = config.filesystem.local.get(fs) {
        let fs = Arc::new(RwLock::new(LocalFS::new()));
        let abs_dir_path = Arc::new(local_fs.dir.add_rel_dir(rel_dir_path));

        Ok(FSMount::new(fs, abs_dir_path))
    } else if let Some(webdav_fs) = config.filesystem.webdav.get(fs) {
        match get_password(&webdav_fs.password_id) {
            Ok(password) => {
                let fs = Arc::new(RwLock::new(WebDAVFS::new(
                    &webdav_fs.user,
                    &password,
                    webdav_fs.timeout_secs,
                )));

                let abs_dir_path = Arc::new(webdav_fs.url.add_rel_dir(rel_dir_path));
                Ok(FSMount::new(fs, abs_dir_path))
            }
            Err(err) => {
                return Err(Arc::new(err));
            }
        }
    } else {
        return Err(Arc::new(StringError::new(format!(
            "No filesystem with the name {:?} found",
            fs
        ))));
    }
}

/// The cuba api. This provides access to backup, restore, verify and clean to cli or gui.
pub struct Cuba {
    config: Option<Config>,
    sender: Sender<Arc<dyn Message>>,
}

impl Cuba {
    /// Creates an api instance with a message sender.
    pub fn new(sender: Sender<Arc<dyn Message>>) -> Self {
        Self {
            config: None,
            sender,
        }
    }

    /// Replace the config from outside.
    pub fn set_config(&mut self, config: Config) {
        self.config = Some(config);
    }

    /// Get immutable reference to config if it exists.
    pub fn config(&self) -> Option<&Config> {
        self.config.as_ref()
    }

    /// Get mutable reference to config if it exists.
    pub fn config_mut(&mut self) -> Option<&mut Config> {
        self.config.as_mut()
    }

    /// Returns the config, if exists.
    pub fn requires_config(&self) -> Option<&Config> {
        match &self.config {
            Some(config) => Some(config),
            None => {
                send_error!(
                    self.sender,
                    StringError::new("A config is required".to_string())
                );
                None
            }
        }
    }

    /// Sets a password for the given id.
    pub fn set_password(&self, id: &str, password: &SecretString) {
        if let Some(config) = self.requires_config() {
            if config.has_password_id(id) {
                match store_password(id, password) {
                    Ok(()) => {
                        send_info!(self.sender, "Password for id {:?} stored", id);
                    }
                    Err(err) => {
                        send_error!(self.sender, err)
                    }
                }
            } else {
                send_error!(
                    self.sender,
                    StringError::new(format!("No password-id {:?} found in config", id))
                );
            }
        }
    }

    /// Deletes the password for the given id.
    pub fn delete_password(&self, id: &str) {
        match remove_password(id) {
            Ok(()) => {
                send_info!(self.sender, "Password for id {:?} deleted", id);
            }
            Err(err) => {
                send_error!(self.sender, err)
            }
        }
    }

    /// Runs the backup with the given backup profile name.
    pub fn run_backup(&self, backup_name: &str) {
        if let Some(config) = self.requires_config() {
            match config.backup.get(backup_name) {
                Some(backup) => {
                    let src_mnt = match create_fs_mount(config, &backup.src_fs, &backup.src_dir) {
                        Ok(mount) => mount,
                        Err(err) => {
                            send_error!(self.sender, err);
                            return;
                        }
                    };

                    let dest_mnt = match create_fs_mount(config, &backup.dest_fs, &backup.dest_dir)
                    {
                        Ok(mount) => mount,
                        Err(err) => {
                            send_error!(self.sender, err);
                            return;
                        }
                    };

                    run_backup(
                        config.transfer_threads,
                        backup.compression,
                        backup.encrypt,
                        &backup.password_id,
                        &backup.include,
                        &backup.exclude,
                        &FSConnection::new(src_mnt, dest_mnt),
                        self.sender.clone(),
                    );
                }
                None => {
                    send_error!(
                        self.sender,
                        StringError::new(format!(
                            "No backup profile with the name {:?} found",
                            backup_name
                        ))
                    );
                }
            }
        }
    }

    /// Runs the restore with the given restore profile name.
    pub fn run_restore(&self, restore_name: &str) {
        if let Some(config) = self.requires_config() {
            match config.restore.get(restore_name) {
                Some(restore) => {
                    let src_mnt = match create_fs_mount(config, &restore.src_fs, &restore.src_dir) {
                        Ok(mount) => mount,
                        Err(err) => {
                            send_error!(self.sender, err);
                            return;
                        }
                    };

                    let dest_mnt =
                        match create_fs_mount(config, &restore.dest_fs, &restore.dest_dir) {
                            Ok(mount) => mount,
                            Err(err) => {
                                send_error!(self.sender, err);
                                return;
                            }
                        };

                    run_restore(
                        config.transfer_threads,
                        &restore.include,
                        &restore.exclude,
                        FSConnection::new(src_mnt, dest_mnt),
                        self.sender.clone(),
                    );
                }
                None => {
                    send_error!(
                        self.sender,
                        StringError::new(format!(
                            "No restore profile with the name {:?} found",
                            restore_name
                        ))
                    );
                }
            }
        }
    }

    /// Runs the verify with the given backup profile name.
    pub fn run_verify(&self, backup_name: &str, verify_all: &bool) {
        if let Some(config) = self.requires_config() {
            match config.backup.get(backup_name) {
                Some(backup) => {
                    let fs_mnt = match create_fs_mount(config, &backup.dest_fs, &backup.dest_dir) {
                        Ok(mount) => mount,
                        Err(err) => {
                            send_error!(self.sender, err);
                            return;
                        }
                    };

                    run_verify(
                        config.transfer_threads,
                        fs_mnt,
                        *verify_all,
                        self.sender.clone(),
                    );
                }
                None => {
                    send_error!(
                        self.sender,
                        StringError::new(format!(
                            "No backup profile with the name {:?} found",
                            backup_name
                        ))
                    );
                }
            }
        }
    }

    /// Runs the clean with the given backup profile name.
    pub fn run_clean(&self, backup_name: &str) {
        if let Some(config) = self.requires_config() {
            match config.backup.get(backup_name) {
                Some(backup) => {
                    let fs_mnt = match create_fs_mount(config, &backup.dest_fs, &backup.dest_dir) {
                        Ok(mount) => mount,
                        Err(err) => {
                            send_error!(self.sender, err);
                            return;
                        }
                    };

                    run_clean(fs_mnt, self.sender.clone());
                }
                None => {
                    send_error!(
                        self.sender,
                        StringError::new(format!(
                            "No backup profile with the name {:?} found",
                            backup_name
                        ))
                    );
                }
            }
        }
    }
}
