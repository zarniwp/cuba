# Cuba LIB 

Cuba is a lightweight and flexible backup tool for your local data. It allows you to back up files to **WebDAV** cloud or network drives while keeping them in their original form by default. Optional **compression** and **encryption** ensure your backups are efficient and secure, and because standard formats are used, your files can also be accessed or restored with public tools if needed.

For more Information, see [Workspace README](../README.md)

## Usage

```rust
use std::sync::Arc;
use crossbeam_channel::{Sender, unbounded};
use cuba_lib::{core::cuba::{Cuba, RunHandle}, 
use cuba_lib::shared::{config::load_config_from_file, message::Message, msg_receiver::MsgReceiver}};

fn main() {
    // Create a channel for communication between the your app and the Cuba instance.
    let (sender, receiver) = unbounded::<Arc<dyn Message>>();

    // Optional: Create a message handler to keep track of messages and progress.
    let my_message_handler = MyMessageHandler::new();

    // Bind the message handler to the receiver channel.
    let msg_receiver = MsgReceiver::new(receiver, my_message_handler);

    // Create a new Cuba instance with the sender channel.
    let mut cuba = Cuba::new(sender.clone());

    // Load the configuration from the file "cuba.toml"
    if let Some(config) = load_config_from_file(sender.clone(), "cuba.toml") {
        cuba.set_config(config);
    }

    // Run a backup with the profile "MyBackup".
    cuba.run_backup(RunHandle::default(), "MyBackup");

    // Create a restore with the profile "MyRestore".
    cuba.run_restore(RunHandle::default(), "MyRestore");
}
```
## License

See [Workspace README](../README.md).