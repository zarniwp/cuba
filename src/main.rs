mod cli;
mod core;
mod shared;

use clap::{CommandFactory, Parser};
use crossbeam_channel::{Sender, unbounded};
use inquire::Password;
use secrecy::SecretString;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::{fs, io};

use crate::core::api::Cuba;
use crate::shared::config::{Config, EXAMPLE_CONFIG};
use crate::shared::message::StringError;
use crate::shared::message::{Message, MsgDispatcher};

use crate::cli::cli_cmds::{
    Cli, ConfigCommands, ConfigExampleCommands, MainCommands, PasswordCommands,
};
use crate::cli::console_out::ConsoleOut;
use crate::cli::file_logger::Log;
use crate::cli::file_logger::LogBuilder;
use crate::cli::progress_bars::ProgressBars;

/// A macro the subscribes the `Log` to the `MsgDispatcher`.
macro_rules! use_logger {
    ($msg_logger:ident, $msg_dispatcher:expr) => {{
        let logger_receiver = $msg_dispatcher.subscribe();
        let logger = LogBuilder::new(Arc::new(logger_receiver))
            .add_log_file(vec![log::Level::Info], "cuba.info.log")
            .add_log_file(vec![log::Level::Warn], "cuba.warn.log")
            .add_log_file(vec![log::Level::Error], "cuba.error.log")
            .build();
        $msg_logger = Some(logger);

        if let Some(logger) = $msg_logger.as_mut() {
            logger.start();
        }
    }};
}

/// A macro the unsubscribes the `Log` from the `MsgDispatcher`.
macro_rules! unuse_logger {
    ($msg_logger:ident, $msg_dispatcher:expr) => {{
        if let Some(mut logger) = $msg_logger.take() {
            logger.stop();
        }
    }};
}

/// A macro the subscribes the `ConsoleOut` to the `MsgDispatcher`.
macro_rules! use_console_out {
    ($msg_console_out:ident, $msg_dispatcher:expr) => {{
        let console_out_receiver = $msg_dispatcher.subscribe();
        $msg_console_out = Some(ConsoleOut::new(Arc::new(console_out_receiver)));

        if let Some(console_out) = $msg_console_out.as_mut() {
            console_out.start();
        }
    }};
}

/// A macro the unsubscribes the `ConsoleOut` from the `MsgDispatcher`.
macro_rules! unuse_console_out {
    ($msg_console_out:ident, $msg_dispatcher:expr) => {{
        if let Some(mut console_out) = $msg_console_out.take() {
            console_out.stop();
        }
    }};
}

/// A macro the subscribes the `ProgressBars` to the `MsgDispatcher`.
macro_rules! use_progress {
    ($msg_progress_bars:ident, $msg_dispatcher:expr, $threads:expr) => {{
        let progress_receiver = $msg_dispatcher.subscribe();
        $msg_progress_bars = Some(ProgressBars::new(Arc::new(progress_receiver), $threads));

        if let Some(progress) = $msg_progress_bars.as_mut() {
            progress.start();
        }
    }};
}

/// A macro the unsubscribes the `ProgressBars` from the `MsgDispatcher`.
macro_rules! unuse_progress {
    ($msg_progress_bars:ident, $msg_dispatcher:expr) => {{
        if let Some(mut progress) = $msg_progress_bars.take() {
            progress.stop();
        }
    }};
}

/// A prompt for setting the password.
fn prompt_password(sender: Sender<Arc<dyn Message>>) -> String {
    loop {
        let password_input = Password::new("Enter your password:")
            .without_confirmation()
            .prompt();

        let password = match password_input {
            Ok(password_ok) if !password_ok.is_empty() => password_ok,
            _ => {
                send_error!(
                    sender.clone(),
                    StringError::new("Password cannot be empty. Try again.".to_string())
                );
                continue;
            }
        };

        let confirm_input = Password::new("Confirm your password:")
            .without_confirmation()
            .prompt();

        match confirm_input {
            Ok(confirm) if confirm == password => return password,
            Ok(_) => send_error!(
                sender.clone(),
                StringError::new("Passwords do not match. Try again.".to_string())
            ),
            Err(_) => send_error!(
                sender.clone(),
                StringError::new("Failed to read confirmation. Try again.".to_string())
            ),
        }
    }
}

/// Writes the example config to the cuba.toml.
pub fn write_example_config(sender: Sender<Arc<dyn Message>>) {
    let path = Path::new("cuba.toml");

    if path.exists() {
        print!("cuba.toml already exists. Overwrite? [y/N]: ");
        if let Err(error) = io::stdout().flush() {
            send_error!(sender.clone(), error);
            return;
        }

        let mut input = String::new();
        if let Err(error) = io::stdin().read_line(&mut input) {
            send_error!(sender.clone(), error);
            return;
        }

        let trimmed = input.trim().to_lowercase();
        if trimmed != "y" && trimmed != "yes" {
            send_error!(
                sender.clone(),
                StringError::new("Aborted. Existing file was not overwritten.".to_string())
            );
            return;
        }
    }

    match fs::write(path, EXAMPLE_CONFIG) {
        Ok(_) => send_info!(sender, "Example config written to cuba.toml"),
        Err(error) => send_error!(sender.clone(), error),
    }
}

/// Load config from disk.
pub fn load_config_from_file(sender: Sender<Arc<dyn Message>>, path: &str) -> Option<Config> {
    match std::fs::read_to_string(path) {
        Ok(content) => match toml::from_str::<Config>(&content) {
            Ok(config) => Some(config),
            Err(err) => {
                send_error!(sender, err);
                None
            }
        },
        Err(err) => {
            send_error!(sender, err);
            None
        }
    }
}

fn main() {
    let (sender, receiver) = unbounded::<Arc<dyn Message>>();
    let arc_receiver = Arc::new(receiver);

    let mut msg_dispatcher = MsgDispatcher::new(arc_receiver);

    msg_dispatcher.start();

    #[allow(unused_assignments)]
    let mut msg_console_out: Option<ConsoleOut> = None;
    #[allow(unused_assignments)]
    let mut msg_logger: Option<Log> = None;
    #[allow(unused_assignments)]
    let mut msg_progress_bars: Option<ProgressBars> = None;

    use_logger!(msg_logger, msg_dispatcher);
    use_console_out!(msg_console_out, msg_dispatcher);

    // Show help if no arguments are passed.
    if std::env::args().len() == 1 {
        Cli::command().print_help().unwrap();
    } else {
        let mut cuba = Cuba::new(sender.clone());

        if let Some(config) = load_config_from_file(sender.clone(), "cuba.toml") {
            cuba.set_config(config);
        }

        match Cli::try_parse() {
            Ok(cli) => match &cli.command {
                MainCommands::Backup { backup } => {
                    if let Some(config) = cuba.requires_config() {
                        send_info!(sender, "Start backup of {:?}", backup);
                        unuse_console_out!(msg_console_out, msg_dispatcher);
                        use_progress!(msg_progress_bars, msg_dispatcher, config.transfer_threads);

                        cuba.run_backup(backup);

                        unuse_progress!(msg_progress_bars, msg_dispatcher);
                        use_console_out!(msg_console_out, msg_dispatcher);
                        send_info!(sender, "Backup finished");
                    }
                }
                MainCommands::Restore { restore } => {
                    if let Some(config) = cuba.requires_config() {
                        send_info!(sender, "Start restore of {:?}", restore);
                        unuse_console_out!(msg_console_out, msg_dispatcher);
                        use_progress!(msg_progress_bars, msg_dispatcher, config.transfer_threads);

                        cuba.run_restore(restore);

                        unuse_progress!(msg_progress_bars, msg_dispatcher);
                        use_console_out!(msg_console_out, msg_dispatcher);
                        send_info!(sender, "Restore finished");
                    }
                }
                MainCommands::Verify { backup, all } => {
                    if let Some(config) = cuba.requires_config() {
                        send_info!(sender, "Start verify of {:?}", backup);
                        unuse_console_out!(msg_console_out, msg_dispatcher);
                        use_progress!(msg_progress_bars, msg_dispatcher, config.transfer_threads);

                        cuba.run_verify(backup, all);

                        unuse_progress!(msg_progress_bars, msg_dispatcher);
                        use_console_out!(msg_console_out, msg_dispatcher);
                        send_info!(sender, "Verify finished");
                    }
                }
                MainCommands::Clean { backup } => {
                    if let Some(config) = cuba.requires_config() {
                        send_info!(sender, "Start clean of {:?}", backup);
                        unuse_console_out!(msg_console_out, msg_dispatcher);
                        use_progress!(msg_progress_bars, msg_dispatcher, config.transfer_threads);

                        cuba.run_clean(backup);

                        unuse_progress!(msg_progress_bars, msg_dispatcher);
                        use_console_out!(msg_console_out, msg_dispatcher);
                        send_info!(sender, "Clean finished");
                    }
                }
                MainCommands::Password { command } => match command {
                    PasswordCommands::Set { id } => {
                        let password = prompt_password(sender);
                        cuba.set_password(id, &SecretString::from(password));
                    }
                    PasswordCommands::Delete { id } => {
                        cuba.delete_password(id);
                    }
                },
                MainCommands::Config { command } => match command {
                    ConfigCommands::Example { command } => match command {
                        ConfigExampleCommands::Show => {
                            println!("{}", EXAMPLE_CONFIG);
                        }
                        ConfigExampleCommands::Write => {
                            write_example_config(sender);
                        }
                    },
                },
            },
            Err(err) => {
                send_error!(sender.clone(), StringError::new(format!("{}", err)));
            }
        }
    }

    unuse_logger!(msg_logger, msg_dispatcher);
    unuse_console_out!(msg_console_out, msg_dispatcher);
    msg_dispatcher.stop();
}
