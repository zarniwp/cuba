# Cuba LIB 

Core library for Cuba.

For further information, see [Workspace README](../README.md)

# Installation

Install as a dependency:
```bash
cargo add cuba-lib
```

## Usage

```rust
use std::sync::Arc;
use crossbeam_channel::unbounded;
use cuba_lib::{core::cuba::{Cuba, RunHandle}, shared::config::{EXAMPLE_CONFIG, save_config_to_file}};
use cuba_lib::shared::{config::load_config_from_file, message::Message, msg_receiver::MsgReceiver};

fn main() {
    // Create a channel for communication between your app and the Cuba instance.
    let (sender, receiver) = unbounded::<Arc<dyn Message>>();

    // Optional: Create a message handler to keep track of messages and progress.
    let my_message_handler = MyMessageHandler::new();

    // Bind the message handler to the receiver channel.
    let msg_receiver = MsgReceiver::new(receiver, my_message_handler);

    // Create a new Cuba instance with the sender channel.
    let mut cuba = Cuba::new(sender.clone());

    // Write the example config to "cuba.toml", if it doesn't exist.
    if !Path::new("cuba.toml").exists() {
        let example_config = load_config_from_str(sender, path, EXAMPLE_CONFIG);
        save_config_to_file(sender, "cuba.toml", example_config);
    }

    // Load the configuration from the file "cuba.toml"
    if let Some(config) = load_config_from_file(sender.clone(), "cuba.toml") {
        cuba.set_config(config);
    }

    // Run a backup with the profile "MyBackup".
    cuba.run_backup(RunHandle::default(), "MyBackup");

    // Create a restore with the profile "MyRestore".
    cuba.run_restore(RunHandle::default(), "MyRestore");

    // Run a verify (new files) with the profile "MyBackup"
    cuba.run_verify(RunHandle::default(), "MyBackup", &false);

    // Run a verify (all files) with the profile "MyBackup"
    cuba.run_verify(RunHandle::default(), "MyBackup", &true);

    // Run a clean with the profile "MyBackup".
    cuba.run_clean(RunHandle::default(), "MyBackup");
}
```
## License

See [Workspace README](../README.md).