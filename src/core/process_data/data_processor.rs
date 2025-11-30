use crossbeam_channel::Sender;
use std::{io::Read, sync::Arc};

use crate::shared::{
    message::Message,
    npath::{File, NPath, Rel},
};

/// The `DataProcessor` type.
pub type DataProcessor = Arc<
    dyn Fn(
            Sender<Arc<dyn Message>>,
            Box<dyn Read + Send>,
            Option<&mut NPath<Rel, File>>,
        ) -> Box<dyn Read + Send>
        + Send
        + Sync,
>;
