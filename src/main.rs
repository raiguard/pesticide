mod adapter;
mod config;
mod controller;
mod dap_types;
mod ui;

#[macro_use]
extern crate log;

use crate::adapter::Adapter;
use crate::config::Config;
use crate::ui::Ui;
use anyhow::{anyhow, Context, Result};
use pico_args::Arguments;
use simplelog::{Config as SLConfig, LevelFilter, WriteLogger};
use std::fs::File;
use std::path::PathBuf;

fn main() -> Result<()> {
    // Parse CLI arguments
    let mut args = Arguments::from_env();
    if args.contains("--help") {
        println!("{}", HELP);
        return Ok(());
    }
    let cli = Cli {
        config: args.opt_value_from_str("--config")?,
        log: args.opt_value_from_str("--log")?,
    };

    // Initialize logging
    let path = if let Some(path) = &cli.log {
        path.clone()
    } else {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow!("Could not resolve OS data directory"))?
            .join("pesticide");
        if !data_dir.exists() {
            std::fs::create_dir(data_dir.clone())?;
        }
        data_dir.join("pesticide.log")
    };
    WriteLogger::init(LevelFilter::Trace, SLConfig::default(), File::create(path)?)?;

    debug!("{:?}", cli);

    // Retrieve local configuration
    let config = Config::new(cli).context("Invalid configuration file")?;

    // TODO: Decide to daemonize into a server, send a command to the server, or become a client

    // Initialize UI
    let ui = Ui::new()?;

    // Initialize adapter
    let adapter = Adapter::new(config)?;

    // Start debugging session
    controller::start(adapter, ui)?;

    Ok(())
}

#[derive(Debug)]
pub struct Cli {
    config: Option<PathBuf>,
    log: Option<PathBuf>,
}

const HELP: &str = "\
usage: pesticide [options]
options:
    --config <PATH>  Path to the pesticide.toml file (defaults to $PWD/pesticide.toml)
    --help           Print help information
    --log <PATH>     Write log to the given file (defaults to $HOME/.local/share/pesticide/pesticide.log)";
