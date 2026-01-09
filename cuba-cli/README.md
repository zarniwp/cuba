# Cuba CLI

Cuba is a lightweight and flexible backup tool for your local data. It allows you to back up files to **WebDAV** cloud or network drives while keeping them in their original form by default. Optional **compression** and **encryption** ensure your backups are efficient and secure, and because standard formats are used, your files can also be accessed or restored with public tools if needed.

For further information, see [Workspace README](../README.md)

## Usage

```bash
Cuba - a lightweight backup tool

Usage: cuba <COMMAND>

Commands:
  backup    Run a backup
  restore   Run a restore
  verify    Run a verify
  clean     Run a clean
  password  Manage passwords
  config    Show/write config
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Quick Start

First, create an example configuration:

```bash
$ cuba config example write
```

Then open the generated config file and define:
* a local filesystem (the source of your data)
* a backup filesystem (e.g., WebDAV, local path, etc.)
* a backup profile describing what should be backed up and where

If you want to use encryption, store a password in your OS keyring:

```bash
$ cuba password set backup_id
```

Make sure that backup_id matches the password_id used in your encryption settings.

## License

See [Workspace README](../README.md).