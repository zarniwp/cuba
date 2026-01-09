# Cuba LIB 

Cuba is a lightweight and flexible backup tool for your local data. It allows you to back up files to **WebDAV** cloud or network drives while keeping them in their original form by default. Optional **compression** and **encryption** ensure your backups are efficient and secure, and because standard formats are used, your files can also be accessed or restored with public tools if needed.

For more Information, see [Workspace README](../README.md)

## Usage

```rust
use crossbeam_channel::{Sender, unbounded};
use cuba_lib::core::cuba::{Cuba, RunHandle};

main() {
    // Create a channel for communication between the GUI and the Cuba instance
    let (sender, receiver) = unbounded::<Arc<dyn Message>>();

    // Create a new Cuba instance with the sender channel
    let mut cuba = Cuba::new(sender.clone());

    // Load the configuration from the file "cuba.toml"
    if let Some(config) = load_config_from_file(sender.clone(), "cuba.toml") {
        cuba.set_config(config);
    }

    // Create a new backup from the profile "MyBackup".
    cuba.run_backup(RunHandle::default(), "MyBackup");
}
```
## License

See [Workspace README](../README.md).