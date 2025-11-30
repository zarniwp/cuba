use age::secrecy::SecretString;
use age::{Decryptor, Encryptor};
use std::io::{Read, pipe};
use std::sync::Arc;
use std::{iter, thread};

use crossbeam_channel::Sender;

use crate::send_error;
use crate::shared::message::Message;
use crate::shared::npath::{File, NPath, Rel};

use super::data_processor::DataProcessor;

const AGE_WORK_FACTOR: u8 = 14;

/// Encrypt data processor for age.
pub fn age_encrypt_proc(password: SecretString) -> DataProcessor {
    Arc::new(
        move |sender: Sender<Arc<dyn Message>>,
              mut input: Box<dyn Read + Send>,
              dest_rel_path: Option<&mut NPath<Rel, File>>|
              -> Box<dyn Read + Send> {
            // Create a pipe: writer for encryption output, reader for caller.
            match pipe() {
                Ok((reader, mut writer)) => {
                    // Clone password for thread.
                    let password = password.clone();
                    let sender_clone = sender.clone();

                    // Create a recipient with a specific work factor.
                    let mut recipient = age::scrypt::Recipient::new(password.clone());
                    recipient.set_work_factor(AGE_WORK_FACTOR);

                    // Spawn background thread for encryption.
                    thread::spawn(move || {
                        match Encryptor::with_recipients(iter::once(&recipient as _)) {
                            Ok(encryptor) => match encryptor.wrap_output(&mut writer) {
                                Ok(mut encrypt_writer) => {
                                    if let Err(err) = std::io::copy(&mut input, &mut encrypt_writer)
                                    {
                                        send_error!(sender_clone, err);
                                        return;
                                    }
                                    if let Err(err) = encrypt_writer.finish() {
                                        send_error!(sender_clone, err);
                                    }
                                }
                                Err(err) => {
                                    send_error!(sender_clone, err);
                                }
                            },
                            Err(err) => {
                                send_error!(sender_clone, err);
                            }
                        }
                    });

                    // Push extension.
                    if let Some(dest_rel_path) = dest_rel_path {
                        dest_rel_path.push_extension("age");
                    }

                    // Return the reader immediately; encryption happens in background.
                    Box::new(reader)
                }
                Err(err) => {
                    send_error!(sender, err);
                    // Return an empty reader so pipeline can continue.
                    Box::new(std::io::empty())
                }
            }
        },
    )
}

/// Dencrypt data processor for age.
pub fn age_decrypt_proc(password: SecretString) -> DataProcessor {
    Arc::new(
        move |sender: Sender<Arc<dyn Message>>,
              input: Box<dyn Read + Send>,
              dest_rel_path: Option<&mut NPath<Rel, File>>|
              -> Box<dyn Read + Send> {
            // Try to create decryptor.
            let decryptor = match Decryptor::new(input) {
                Ok(decryptor) => decryptor,
                Err(err) => {
                    send_error!(sender, err);
                    return Box::new(std::io::empty()); // return dummy reader
                }
            };

            // Create an identity with a specific work factor.
            let mut identity = age::scrypt::Identity::new(password.clone());
            identity.set_max_work_factor(AGE_WORK_FACTOR);

            // Try to create decrypted reader.
            let reader = match decryptor.decrypt(iter::once(&identity as _)) {
                Ok(reader) => reader,
                Err(err) => {
                    send_error!(sender, err);
                    return Box::new(std::io::empty()); // return dummy reader
                }
            };

            // Pop extension.
            if let Some(dest_rel_path) = dest_rel_path {
                dest_rel_path.pop_extension_if("age");
            }

            // Return the reader.
            Box::new(reader)
        },
    )
}
