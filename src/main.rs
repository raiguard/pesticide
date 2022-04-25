#[macro_use]
extern crate log;

use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use pico_args::Arguments;
use simplelog::ColorChoice;
use simplelog::Config as SLConfig;
use simplelog::LevelFilter;
use simplelog::TermLogger;
use simplelog::TerminalMode;
use simplelog::WriteLogger;
use std::fs::File;
use std::path::PathBuf;

mod config;
mod ui;

use config::Config;

fn main() -> Result<()> {
    // CLI arguments
    let mut args = Arguments::from_env();
    // Print help information
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
    let config_path = if let Some(config) = cli.config {
        config
    } else {
        std::env::current_dir()?.join("pesticide.toml")
    };
    if !config_path.exists() {
        bail!(
            "Pesticide configuration not found at {}",
            config_path.to_str().unwrap()
        );
    }
    // TODO: Support multiple debug profiles per config
    let config: Config = toml::from_str(&std::fs::read_to_string(config_path)?)
        .context("Failed to parse configuration file")?;
    debug!("{:#?}", config);

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
