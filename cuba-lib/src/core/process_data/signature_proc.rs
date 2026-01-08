use crossbeam_channel::Sender;

use std::{
    io::Read,
    sync::{Arc, Mutex},
};

use crate::shared::{
    message::Message,
    npath::{File, NPath, Rel},
};

use super::data_processor::DataProcessor;

use blake3;

/// Defines a `HashingReader`.
///
/// A reader that computes a BLAKE3 hash of the data read.
struct HashingReader<R: Read + Send> {
    inner: R,
    hasher: blake3::Hasher,
    output: Arc<Mutex<[u8; 32]>>,
}

/// Methods of `HashingReader`.
impl<R: Read + Send> HashingReader<R> {
    /// Creates a new `HashingReader`.
    fn new(inner: R, output: Arc<Mutex<[u8; 32]>>) -> Self {
        Self {
            inner,
            hasher: blake3::Hasher::new(),
            output,
        }
    }
}

/// Impl of `Read` for `HashingReader`.
impl<R: Read + Send> Read for HashingReader<R> {
    /// Reads data from the inner reader and updates the hash.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes = self.inner.read(buf)?;
        if bytes > 0 {
            self.hasher.update(&buf[..bytes]);
        }
        Ok(bytes)
    }
}

/// Impl of `Drop` for `HashingReader`.
impl<R: Read + Send> Drop for HashingReader<R> {
    fn drop(&mut self) {
        // Compute and write the final hash when dropped.
        let hash = self.hasher.finalize();
        let bytes = hash.as_bytes();
        if let Ok(mut guard) = self.output.lock() {
            guard.copy_from_slice(bytes);
        }
    }
}

/// Creates a data processor that computes the BLAKE3 signature of the data read.
pub fn signature_proc(signature: Arc<Mutex<[u8; 32]>>) -> DataProcessor {
    Arc::new(
        move |_sender: Sender<Arc<dyn Message>>,
              input: Box<dyn Read + Send>,
              _dest_rel_path: Option<&mut NPath<Rel, File>>|
              -> Box<dyn Read + Send> {
            let reader = HashingReader::new(input, signature.clone());
            Box::new(reader)
        },
    )
}
