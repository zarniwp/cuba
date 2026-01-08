use crossbeam_channel::Sender;
use flate2::{Compression, bufread::GzDecoder, write::GzEncoder};
use lazy_static::lazy_static;
use std::{
    io::{BufReader, BufWriter},
    sync::Arc,
};

use crate::{
    core::{fs::fs_base::FSMount, transferred_node::TransferredNodes},
    send_error,
    shared::{
        message::Message,
        npath::{Abs, File, NPath, Rel},
    },
};

// The cuba json as rel path.
lazy_static! {
    pub static ref CUBA_JSON_REL_PATH: NPath<Rel, File> =
        NPath::<Rel, File>::try_from("cuba.json.gz").unwrap();
}

/// Read the cuba json.
pub fn read_cuba_json(
    fs_mnt: &FSMount,
    sender: &Sender<Arc<dyn Message>>,
) -> Option<TransferredNodes> {
    // Create cuba json abs path.
    let cuba_json_abs_path: NPath<Abs, File> =
        fs_mnt.abs_dir_path.add_rel_file(&CUBA_JSON_REL_PATH);

    match fs_mnt.fs.read().unwrap().read_data(&cuba_json_abs_path) {
        Ok(reader) => {
            // Create buf reader.
            let buf_reader = BufReader::new(reader);

            // Create decoder
            let decoder = GzDecoder::new(buf_reader);

            // Read data.
            return match serde_json::from_reader(decoder) {
                Ok(transferred_nodes) => Some(transferred_nodes),
                Err(err) => {
                    send_error!(sender, err);
                    None
                }
            };
        }
        Err(err) => {
            send_error!(sender, err);
        }
    }

    None
}

/// Write the cuba json.
pub fn write_cuba_json(
    fs_mnt: &FSMount,
    transferred_node: &TransferredNodes,
    sender: &Sender<Arc<dyn Message>>,
) {
    // Create cuba json abs path.
    let cuba_json_abs_path: NPath<Abs, File> =
        fs_mnt.abs_dir_path.add_rel_file(&CUBA_JSON_REL_PATH);

    // Write cuba json.
    match fs_mnt.fs.read().unwrap().write_data(&cuba_json_abs_path) {
        Ok(writer) => {
            // Create buf writer.
            let buf_writer = BufWriter::new(writer);

            // Create encoder.
            let encoder = GzEncoder::new(buf_writer, Compression::default());

            // Write data.
            match serde_json::to_writer(encoder, transferred_node) {
                Ok(()) => (),
                Err(err) => send_error!(sender, err),
            }
        }
        Err(err) => {
            send_error!(sender, err);
        }
    }
}
