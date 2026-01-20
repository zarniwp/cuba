use clap::{ArgAction, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cuba",
    version = "1.0",
    author = "Stefan",
    about = "Cuba - a lightweight backup tool"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: MainCommands,
}

#[derive(Subcommand)]
pub enum MainCommands {
    /// Run a backup
    Backup {
        /// The name of the backup profile.
        backup: String,
    },
    /// Run a restore
    Restore {
        /// The name of the restore profile.
        restore: String,
    },
    /// Run a verify
    Verify {
        /// The name of the backup profile.
        backup: String,

        /// Verify all files.
        #[arg(long, action = ArgAction::SetTrue)]
        all: bool,
    },
    /// Run a clean
    Clean {
        /// The name of the backup profile.
        backup: String,
    },
    /// Manage passwords.
    Password {
        #[command(subcommand)]
        command: PasswordCommands,
    },
    /// Show/write config.
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
pub enum PasswordCommands {
    /// Sets a password.
    Set {
        /// The password id.
        id: String,
    },
    /// Deletes a password.
    Delete {
        /// The password id.
        id: String,
    },
    /// Lists the password ids.
    List,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// A config example.
    Example {
        #[command(subcommand)]
        command: ConfigExampleCommands,
    },
}

#[derive(Subcommand)]
pub enum ConfigExampleCommands {
    /// Show config example.
    Show,
    /// Write config example.
    Write,
}
