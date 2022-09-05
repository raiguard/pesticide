// ARCHITECTURE NOTES:
//
// Two primary interaction modes: CLI and TUI
// CLI will accept commands
// TUI will process keybindings, execute the corresponding commands in the background, and present the current data
// The user can remap TUI keybindings to aritrary commands
//
// The server will provide a socket in $XDG_RUNTIME_DIR/pesticide, keyed by provided session name or PID
// External editors may send pesticide commands to this socket
// Data returned from commands will be returned through the socket, and a connected TUI will update as well
// The socket will also accept commands and return responses in JSON format if desired
//
// When pesticide is first opened, it will be in "configuration" mode, where you can set breakpoints and settings before
// starting the debug session
// Somewhat similar to GDB in that respect
// Perhaps we should persist breakpoints across sessions?

mod adapter;
mod config;
mod dap;
mod kakoune;
mod server;
mod ui;

#[macro_use]
extern crate log;

use anyhow::{bail, Result};
use config::Config;
use pico_args::Arguments;
use std::fs::File;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let mut args = Arguments::from_env();
    if args.contains("--help") {
        println!("{HELP}");
        return Ok(());
    } else if args.contains("--kakoune") {
        let kakscript = include_str!("../rc/pesticide.kak");
        println!("{kakscript}");
        return Ok(());
    }
    let cli = Cli {
        config: args
            .opt_value_from_str("--config")?
            .unwrap_or_else(|| std::env::current_dir().unwrap().join("pesticide.toml")),
        request: args.opt_value_from_str("--request")?,
        session: args.opt_value_from_str("--session")?,
    };
    if cli.request.is_some() && cli.session.is_none() {
        bail!("--request flag requires --session to be defined.");
    }
    let config = Config::new(cli.config);
    let session = cli
        .session
        .or_else(|| {
            config
                .as_ref()
                .ok()
                .and_then(|config| config.session_name.clone())
        })
        .unwrap_or_else(|| std::process::id().to_string());

    // Determine named pipe path
    let sock_path = dirs::runtime_dir()
        .expect("Could not get runtime directory")
        .join("pesticide");
    if !sock_path.exists() {
        tokio::fs::create_dir_all(&sock_path).await?;
    }
    let sock_path = sock_path.join(&session);

    if let Some(request) = cli.request {
        let mut socket = UnixStream::connect(&sock_path).await?;
        socket.write_all(request.as_bytes()).await?;
        Ok(())
    } else {
        let config = config?;
        // Create log file
        let log = dirs::data_dir()
            .expect("Unable to get local data directory")
            .join("pesticide");
        if !log.exists() {
            tokio::fs::create_dir_all(&log).await?;
        }
        let log = log.join(format!("{session}.log"));

        // Initialize logging
        simplelog::WriteLogger::init(
            log::LevelFilter::Trace,
            simplelog::Config::default(),
            File::create(log)?,
        )?;

        // Run application
        server::run(config, sock_path, session).await
    }
}

#[derive(Debug)]
pub struct Cli {
    config: PathBuf,
    request: Option<String>,
    session: Option<String>,
}

const HELP: &str = "\
usage: pesticide [options]
options:
    --config <FILE>   Debugger configuration file (default: $PWD/pesticide.toml)
    --help            Print help information
    --kakoune         Print kakoune definitions
    --request <DATA>  Send a request to the given session
    --session <NAME>  Set a session name (default: PID)
";
