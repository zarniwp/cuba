#![allow(unused)]

use argon2::{self, Argon2};
use crossbeam_channel::Sender;
use rand::{RngCore, rng};
use secrecy::{ExposeSecret, SecretString};
use std::io::Cursor;
use std::{io::Read, sync::Arc};

use crate::send_error;
use crate::shared::{
    message::{Message, StringError},
    npath::{File, NPath, Rel},
};

use super::cipher::{Decryptor, Encryptor};
use super::data_processor::DataProcessor;

const SALT_SIZE: usize = 16; // Standard size for Argon2 salt
const KEY_SIZE: usize = 32; // AES256 requires a 32-byte key

/// Derives a key from the password using Argon2.
fn derive_key(
    password: &SecretString,
    salt: &[u8; SALT_SIZE],
) -> Result<[u8; KEY_SIZE], argon2::Error> {
    let argon2 = Argon2::default();

    let mut key_bytes = [0u8; KEY_SIZE]; // The output key material, sized to 32 bytes for AES-256.
    argon2.hash_password_into(password.expose_secret().as_bytes(), salt, &mut key_bytes)?;

    Ok(key_bytes)
}

/// Encrypts the input data and prepends the salt to the ciphertext.
pub fn encrypt_proc(password: SecretString) -> DataProcessor {
    Arc::new(
        move |sender: Sender<Arc<dyn Message>>,
              input: Box<dyn Read + Send>,
              dest_rel_path: Option<&mut NPath<Rel, File>>|
              -> Box<dyn Read + Send> {
            // Generate a random salt.
            let mut salt = [0u8; SALT_SIZE];
            rng().fill_bytes(&mut salt);

            // Derive the encryption key from the password and the salt.
            match derive_key(&password, &salt) {
                Ok(key_bytes) => {
                    // Create an Encryptor instance
                    let encryptor = Encryptor::new(input, key_bytes);

                    // Push extension.
                    if let Some(dest_rel_path) = dest_rel_path {
                        dest_rel_path.push_extension("encrypted");
                    }

                    // Return the encryptor wrapped with the salt.
                    Box::new(SaltPrependingReader::new(Box::new(encryptor), salt))
                }
                Err(err) => {
                    send_error!(sender, StringError::new(err.to_string()));

                    // Return an empty reader so pipeline can continue
                    Box::new(std::io::empty())
                }
            }
        },
    )
}

/// Decrypts the input data by first reading the salt, then using the password to generate the key.
pub fn decrypt_proc(password: SecretString) -> DataProcessor {
    Arc::new(
        move |sender: Sender<Arc<dyn Message>>,
              mut input: Box<dyn Read + Send>,
              dest_rel_path: Option<&mut NPath<Rel, File>>|
              -> Box<dyn Read + Send> {
            // Read the salt from the beginning of the input data.
            let mut salt = [0u8; SALT_SIZE];
            input.read_exact(&mut salt);

            // Derive the encryption key from the password and the salt.
            match derive_key(&password, &salt) {
                Ok(key_bytes) => {
                    // Create a Decryptor instance
                    let decryptor = Decryptor::new(input, key_bytes);

                    // Pop extension.
                    if let Some(dest_rel_path) = dest_rel_path {
                        dest_rel_path.pop_extension_if("encrypted");
                    }

                    // Return the decryptor.
                    Box::new(decryptor)
                }
                Err(err) => {
                    send_error!(sender, StringError::new(err.to_string()));
                    // Return an empty reader so pipeline can continue.
                    Box::new(std::io::empty())
                }
            }
        },
    )
}

/// A reader that prepends a salt to the data read from the underlying reader.
pub struct SaltPrependingReader<R: Read> {
    reader: R,
    salt: [u8; SALT_SIZE],
    salt_read: bool, // Flag to ensure salt is only prepended once.
}

impl<R: Read> SaltPrependingReader<R> {
    /// Creates a new `SaltPrependingReader`.
    pub fn new(reader: R, salt: [u8; SALT_SIZE]) -> Self {
        Self {
            reader,
            salt,
            salt_read: false,
        }
    }
}

/// Impl of `Read` for `SaltPrependingReader`.
impl<R: Read> Read for SaltPrependingReader<R> {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        // First read the salt once, then allow subsequent reads of data.
        if !self.salt_read {
            let salt_len = self.salt.len();
            // Copy the salt into the beginning of the buffer.
            let bytes_to_copy = into.len().min(salt_len);
            into[..bytes_to_copy].copy_from_slice(&self.salt[..bytes_to_copy]);
            self.salt_read = true;

            // If the entire salt fits into the buffer, return immediately.
            if bytes_to_copy == salt_len {
                return Ok(bytes_to_copy);
            }

            // Otherwise, continue to read the data after the salt.
            let bytes_read = self.reader.read(&mut into[bytes_to_copy..])?;

            Ok(bytes_to_copy + bytes_read)
        } else {
            // After the salt has been read, just read the normal data.
            self.reader.read(into)
        }
    }
}
