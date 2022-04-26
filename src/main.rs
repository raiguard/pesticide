mod adapter;
mod config;
mod dap_types;

#[macro_use]
extern crate log;

use anyhow::Result;
use pico_args::Arguments;
use simplelog::{
    ColorChoice, Config as SLConfig, LevelFilter, TermLogger, TerminalMode, WriteLogger,
};
use std::fs::File;
use std::path::PathBuf;

use crate::adapter::Adapter;
use crate::config::Config;
use crate::dap_types::*;

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
            LevelFilter::Trace,
            SLConfig::default(),
            TerminalMode::Stdout,
            ColorChoice::Auto,
        )?;
    };

    debug!("{:#?}", cli);

    // Retrieve local configuration
    let config = Config::new(&cli.config)?;

    // Initialize adapter
    let mut adapter = Adapter::new(config)?;

    // Handle incoming messages
    for msg in &adapter.rx {
        match msg {
            AdapterMessage::Event(payload) => {
                // TODO: Handle this automatically
                adapter.next_seq = payload.seq + 1;

                if let Some(body) = payload.body {
                    match body {
                        EventBody::Output(body) => match body.category {
                            Some(OutputEventCategory::Telemetry) => {
                                info!("IDGAF about telemetry")
                            } // IDGAF about telemetry
                            _ => info!("Debug adapter message: {}", body.output),
                        },
                    }
                }
            }
            AdapterMessage::Request(_) => todo!(),
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
