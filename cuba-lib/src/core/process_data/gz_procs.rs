use crossbeam_channel::Sender;
use flate2::{Compression, read::GzDecoder, read::GzEncoder};
use std::{io::Read, sync::Arc};

use crate::shared::{
    message::Message,
    npath::{File, NPath, Rel},
};

use super::data_processor::DataProcessor;

/// Encode data processor for gz.
pub fn gz_encode_proc(compression: Compression) -> DataProcessor {
    Arc::new(
        move |_sender: Sender<Arc<dyn Message>>,
              input: Box<dyn Read + Send>,
              dest_rel_path: Option<&mut NPath<Rel, File>>|
              -> Box<dyn Read + Send> {
            let encoder = Box::new(GzEncoder::new(input, compression));

            // Push extension.
            if let Some(dest_rel_path) = dest_rel_path {
                dest_rel_path.push_extension("gz");
            }

            encoder
        },
    )
}

/// Decode data processor for gz.
pub fn gz_decode_proc() -> DataProcessor {
    Arc::new(
        move |_sender: Sender<Arc<dyn Message>>,
              input: Box<dyn Read + Send>,
              dest_rel_path: Option<&mut NPath<Rel, File>>|
              -> Box<dyn Read + Send> {
            let decoder = Box::new(GzDecoder::new(input));

            // Pop extension.
            if let Some(dest_rel_path) = dest_rel_path {
                dest_rel_path.pop_extension_if("gz");
            }

            decoder
        },
    )
}
