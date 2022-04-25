mod adapter;
mod config;
mod dap_types;

#[macro_use]
extern crate log;

use anyhow::Result;
use pico_args::Arguments;
use serde::Deserialize;
use simplelog::{
    ColorChoice, Config as SLConfig, LevelFilter, TermLogger, TerminalMode, WriteLogger,
};
use std::fs::File;
use std::path::PathBuf;

use crate::adapter::Adapter;
use crate::config::Config;
use crate::dap_types::*;
use crate::Event;

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
    if let Some(path) = &cli.log {
        WriteLogger::init(LevelFilter::Debug, SLConfig::default(), File::create(path)?)?;
    } else {
        // let data_dir = dirs::data_dir()
        //     .ok_or_else(|| anyhow!("Could not resolve OS data directory"))?
        //     .join("pesticide");
        // if !data_dir.exists() {
        //     std::fs::create_dir(data_dir.clone())?;
        // }
        // data_dir.join("pesticide.log")
        // TEMPORARY:
        TermLogger::init(
            LevelFilter::Debug,
            SLConfig::default(),
            TerminalMode::Stdout,
            ColorChoice::Auto,
        )?;
    };

    debug!("{:#?}", cli);

    // Retrieve local configuration
    let config = Config::new(&cli.config)?;

    // Initialize adapter
    let adapter = Adapter::new(config)?;

    for msg in adapter.rx {
        match msg["type"].as_str().unwrap() {
            EVENT => match msg["event"].as_str().unwrap() {
                InitializedEvent::NAME => (),
                OutputEvent::NAME => {
                    let body = OutputEventBody::deserialize(&msg["body"]).unwrap();
                    if let Some(category) = body.category {
                        match category {
                            OutputEventCategory::Telemetry => (), // We careth not about telemetry
                            _ => println!("{}", body.output),
                        }
                    }
                }
                _ => error!("Unrecognized event"),
            },
            _ => error!("Unrecognized payload"),
        }
    }

    Ok(())
}

#[derive(Debug)]
struct Cli {
    config: Option<PathBuf>,
    log: Option<PathBuf>,
}

const HELP: &str = "\
usage: pesticide [options]
options:
    --config <PATH>  Path to the pesticide.toml file (defaults to PWD/pesticide.toml)
    --help           Print help information
    --log <PATH>     Write log to the given file (defaults to STDOUT)";
