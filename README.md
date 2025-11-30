# Cuba ‒ a lightweight backup tool

[![Build](https://github.com/zarniwp/cuba/actions/workflows/build.yml/badge.svg)](https://github.com/zarniwp/cuba/actions/workflows/build.yml)
[![License: MIT or Apache 2.0](https://img.shields.io/badge/License-MIT_or_Apache_2.0-blue)](https://opensource.org/licenses/MIT)
---

Cuba is a lightweight and flexible backup tool for your local data. It allows you to back up files to **WebDAV** cloud or network drives while keeping them in their original form by default. Optional **compression** and **encryption** ensure your backups are efficient and secure, and because standard formats are used, your files can also be accessed or restored with public tools if needed.

## Features

- By default, files and directories are copied in their original form.
- Supports **compression** (gzip) and **encryption** (age). Files use standard formats, so they can also be accessed or restored with public tools if desired.
- **Incremental backups**: Only changed files are updated, detected using **BLAKE3** hashes.
- Stores a **JSON metadata file** alongside your backup to track file hashes and states.
- Fully **multithreaded** for better performance.
- Uses a `config.json` file for backup profiles.
- Provides useful commands such as `verify`, `restore`, and `clean` to manage backups.
- Password for encrypion are stored in os keyring.

## CLI

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
This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   https://opensource.org/licenses/MIT)

at your option.