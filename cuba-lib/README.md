# Cuba LIB 

## Usage

```rust
    let mut cuba = Cuba::new(sender.clone());

    if let Some(config) = load_config_from_file(sender.clone(), "cuba.toml") {
        cuba.set_config(config);
    }
```

For further information and license, see [Workspace README](../README.md).