// LOGIC:
// Each debug session has one server and N clients
// Each session has a socket in $XDG_RUNTIME_DIR/pesticide, keyed by session name
// The session name is either provided as a CLI flag or is set to the PID
// Daemon mode will start a server without starting any clients
// Kakoune mode will start a client that is connected to the Kakoune session instead of being a standard client
// Regular clients can take arguments to specify how their UI is configured
// Request mode will construct a client, send a single request, then quit
// The server will manage a log in $HOME/.local/share/pesticide/[session].log
// Clients will send their log messages to the server so they all get written to the same file
// Socket messages will use something similar to the DAP - a content length header, then the data as JSON
//
// IMPLEMENTATION:
// Construct a server and client, and pass simple data back and forth
// Send log messages to server to have a single log file
// Connect existing adapter logic to the server
// Send updated events to clients
// Clients request the data they need
// TUI
//
// SERVER ARCHITECTURE:
// - Dedicated task to read and write the data structures
// - A separate task for each client that uses channels to send and receive data from the data task
// - Separate task to listen to server stdout, or use select! in the management task?
// - Each client task will need to be able to send events to the client when the state updates without user input
//
// CLIENT ARCHITECTURE:
// - Dedicated task to manage I/O with the server
// - select! over client TUI or CLI inputs, and messages received from the server
// - separate tasks for rendering the UI and accepting user input
//
// MVP ARCHITECTURE:
// - Single program, no client/server stuff yet, it adds a ton of complexity
// - Limited UI customization, i.e. lazygit

// TODO: Clean out unwraps

mod adapter;
mod config;
mod controller;
mod dap_types;
mod ui;

#[macro_use]
extern crate log;

use anyhow::Result;
use pico_args::Arguments;
use std::fs::File;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let mut args = Arguments::from_env();
    if args.contains("--help") {
        println!("{}", HELP);
        return Ok(());
    }
    let cli = Cli {
        config: args
            .opt_value_from_str("--config")?
            .unwrap_or_else(|| std::env::current_dir().unwrap().join("pesticide.toml")),
    };

    // Create log file
    let log = dirs::data_dir()
        .expect("Unable to get local data directory")
        .join("pesticide");
    if !log.exists() {
        tokio::fs::create_dir_all(log.clone()).await?;
    }
    let log = log.join("pesticide.log");

    // Initialize logging
    simplelog::WriteLogger::init(
        log::LevelFilter::Trace,
        simplelog::Config::default(),
        File::create(log)?,
    )?;

    controller::run(cli.config).await
}

#[derive(Debug)]
pub struct Cli {
    config: PathBuf,
}

const HELP: &str = "\
usage: pesticide [options]
options:
    --config <FILE>   Debugger configuration file (default: $PWD/pesticide.toml)
    --help            Print help information
    --server          Initialize as server instead of client
    --session <NAME>  Set a session name (default: PID)
";
