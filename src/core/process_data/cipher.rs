#![allow(unused)]

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use rand::{RngCore, rng};
use std::io::{self, Read};
use std::sync::atomic::{AtomicU64, Ordering};

const CHUNK_SIZE: usize = 64 * 1024; // 64 KB for the chunk itself
const TAG_SIZE: usize = 16; // AES-GCM tag size
const NONCE_SIZE: usize = 12; // AES-GCM nonce size

static NONCE_COUNTER: AtomicU64 = AtomicU64::new(0);
lazy_static::lazy_static! {
    static ref NONCE_PREFIX: [u8; 4] = {
        let mut prefix = [0u8; 4];
        rng().fill_bytes(&mut prefix);
        prefix
    };
}

/// Generates a unique 12-byte nonce: 4 random prefix + 8-byte counter.
fn next_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[..4].copy_from_slice(&*NONCE_PREFIX);
    nonce[4..].copy_from_slice(&NONCE_COUNTER.fetch_add(1, Ordering::Relaxed).to_be_bytes());
    nonce
}

/// Encrypts a chunk of data using AES-GCM with a unique 12-byte nonce.
///
/// This function takes in a cipher and a chunk of data, generates a nonce,
/// and returns the encrypted data with the nonce prepended to it.
///
/// # Arguments  
///
/// * `cipher` - The AES-GCM cipher used for encryption.
/// * `chunk` - The data to encrypt.
///
/// # Returns
///
/// Returns a `Vec<u8>` containing the nonce followed by the encrypted data.
/// In case of failure, an `io::Error` is returned.
fn encrypt(cipher: &Aes256Gcm, chunk: &[u8]) -> io::Result<Vec<u8>> {
    let nonce = next_nonce(); // Generate nonce.

    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), chunk)
        .map_err(|_| io::Error::other("Encryption failed"))?;

    // Allocate buffer efficiently
    let mut encrypted_data = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    encrypted_data.extend_from_slice(&nonce); // Append nonce first
    encrypted_data.extend_from_slice(&ciphertext); // Append encrypted data

    Ok(encrypted_data)
}

/// Decrypts a chunk of encrypted data using AES-GCM.
///
/// This function takes in a cipher and a chunk of encrypted data, extracts the nonce
/// and the ciphertext, and returns the decrypted data.
///
/// # Arguments
///
/// * `cipher` - The AES-GCM cipher used for decryption.
/// * `chunk` - The encrypted data to decrypt, which includes the nonce and ciphertext.
///
/// # Returns
///
/// Returns a `Vec<u8>` containing the decrypted data. If the data is invalid or decryption fails,
/// an `io::Error` is returned.
fn decrypt(cipher: &Aes256Gcm, chunk: &[u8]) -> io::Result<Vec<u8>> {
    if chunk.len() < NONCE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Chunk too small",
        ));
    }

    let (nonce_bytes, encrypted_data) = chunk.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, encrypted_data)
        .map_err(|_| io::Error::other("Decryption failed"))
}

/// Struct representing a cipher that processes data in chunks and applies encryption or decryption.
///
/// This struct is used to read data, process it with AES-GCM encryption or decryption,
/// and manage the buffer used to store intermediate data.
struct Cipher<R: Read> {
    reader: R,                                               // The input data reader
    cipher: Aes256Gcm, // The AES-GCM cipher used for encryption/decryption
    buffer: Vec<u8>,   // Buffer to hold processed data
    buffer_pos: usize, // Current position in the buffer
    cipher_fn: fn(&Aes256Gcm, &[u8]) -> io::Result<Vec<u8>>, // The cipher function (encrypt or decrypt)
    chunk_size: usize, // The size of the data chunks to process
}

impl<R: Read> Cipher<R> {
    /// Creates a new `Cipher` instance.
    ///
    /// # Arguments
    ///
    /// * `reader` - The input data reader (e.g., file, network stream, etc.).
    /// * `key_bytes` - The key used to initialize the AES-GCM cipher.
    /// * `cipher_fn` - The function to apply encryption or decryption.
    /// * `chunk_size` - The size of the data chunks to process.
    ///
    /// # Returns
    ///
    /// Returns a new `Cipher` instance configured with the provided parameters.
    fn new(
        reader: R,
        key_bytes: [u8; 32],
        cipher_fn: fn(&Aes256Gcm, &[u8]) -> io::Result<Vec<u8>>,
        chunk_size: usize,
    ) -> Self {
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        Self {
            reader,
            cipher,
            buffer: Vec::new(),
            buffer_pos: 0,
            cipher_fn,
            chunk_size,
        }
    }

    /// Returns a reference to the underlying reader.
    ///
    /// # Returns
    ///
    /// A reference to the reader.
    #[allow(dead_code)] // Suppressing dead code warning for now
    fn get_ref(&self) -> &R {
        &self.reader
    }

    /// Returns a mutable reference to the underlying reader.
    ///
    /// # Returns
    ///
    /// A mutable reference to the reader.
    #[allow(dead_code)] // Suppressing dead code warning for now
    fn get_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    /// Consumes the `Cipher` and returns the underlying reader.
    ///
    /// # Returns
    ///
    /// The underlying reader.
    #[allow(dead_code)] // Suppressing dead code warning for now
    fn into_inner(self) -> R {
        self.reader
    }
}

impl<R: Read> Read for Cipher<R> {
    /// Reads data from the underlying reader, processes it using the cipher, and returns it.
    ///
    /// This function will process data in chunks of the specified size, applying the cipher function
    /// (either encryption or decryption) to each chunk.
    ///
    /// # Arguments
    ///
    /// * `into` - A mutable slice where the processed data will be copied to.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes read and written to the `into` buffer.
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        if self.buffer_pos >= self.buffer.len() {
            let mut chunk = vec![0; self.chunk_size];

            let mut bytes_read = 0;
            while bytes_read < self.chunk_size {
                let bytes_current_read = self.reader.read(&mut chunk[bytes_read..])?;
                if bytes_current_read == 0 {
                    break; // EOF reached
                }
                bytes_read += bytes_current_read;
            }

            if bytes_read == 0 {
                return Ok(0); // EOF reached
            }

            chunk.truncate(bytes_read); // Ensure correct length

            // Process the data
            self.buffer = (self.cipher_fn)(&self.cipher, &chunk)?;
            self.buffer_pos = 0;
        }

        let bytes_to_copy = self.buffer.len() - self.buffer_pos;
        let bytes_to_write = bytes_to_copy.min(into.len());

        // Copy data to output
        into[..bytes_to_write]
            .copy_from_slice(&self.buffer[self.buffer_pos..self.buffer_pos + bytes_to_write]);
        self.buffer_pos += bytes_to_write;

        Ok(bytes_to_write)
    }
}

/// Encryptor struct that wraps around the `Cipher` for encryption.
pub struct Encryptor<R: Read> {
    cipher: Cipher<R>,
}

impl<R: Read> Encryptor<R> {
    /// Creates a new `Encryptor` instance.
    ///
    /// # Arguments
    ///
    /// * `reader` - The input data reader.
    /// * `key_bytes` - The encryption key.
    ///
    /// # Returns
    ///
    /// A new `Encryptor` instance configured with the provided parameters.
    pub fn new(reader: R, key_bytes: [u8; 32]) -> Self {
        Encryptor {
            cipher: Cipher::new(reader, key_bytes, encrypt, CHUNK_SIZE),
        }
    }

    /// Returns a reference to the underlying reader.
    ///
    /// # Returns
    ///
    /// A reference to the reader.
    #[allow(dead_code)] // Suppressing dead code warning for now
    pub fn get_ref(&self) -> &R {
        self.cipher.get_ref()
    }

    /// Returns a mutable reference to the underlying reader.
    ///
    /// # Returns
    ///
    /// A mutable reference to the reader.
    #[allow(dead_code)] // Suppressing dead code warning for now
    pub fn get_mut(&mut self) -> &mut R {
        self.cipher.get_mut()
    }

    /// Consumes the `Encryptor` and returns the underlying reader.
    ///
    /// # Returns
    ///
    /// The underlying reader.
    #[allow(dead_code)] // Suppressing dead code warning for now
    pub fn into_inner(self) -> R {
        self.cipher.into_inner()
    }
}

impl<R: Read> Read for Encryptor<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.cipher.read(into)
    }
}

/// Decryptor struct that wraps around the `Cipher` for decryption.
pub struct Decryptor<R: Read> {
    cipher: Cipher<R>,
}

impl<R: Read> Decryptor<R> {
    /// Creates a new `Decryptor` instance.
    ///
    /// # Arguments
    ///
    /// * `reader` - The input data reader.
    /// * `key_bytes` - The decryption key.
    ///
    /// # Returns
    ///
    /// A new `Decryptor` instance configured with the provided parameters.
    pub fn new(reader: R, key_bytes: [u8; 32]) -> Self {
        Decryptor {
            cipher: Cipher::new(
                reader,
                key_bytes,
                decrypt,
                CHUNK_SIZE + TAG_SIZE + NONCE_SIZE,
            ),
        }
    }

    /// Returns a reference to the underlying reader.
    ///
    /// # Returns
    ///
    /// A reference to the reader.
    #[allow(dead_code)] // Suppressing dead code warning for now
    pub fn get_ref(&self) -> &R {
        self.cipher.get_ref()
    }

    /// Returns a mutable reference to the underlying reader.
    ///
    /// # Returns
    ///
    /// A mutable reference to the reader.
    #[allow(dead_code)] // Suppressing dead code warning for now
    pub fn get_mut(&mut self) -> &mut R {
        self.cipher.get_mut()
    }

    /// Consumes the `Decryptor` and returns the underlying reader.
    ///
    /// # Returns
    ///
    /// The underlying reader.
    #[allow(dead_code)] // Suppressing dead code warning for now
    pub fn into_inner(self) -> R {
        self.cipher.into_inner()
    }
}

impl<R: Read> Read for Decryptor<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.cipher.read(into)
    }
}
