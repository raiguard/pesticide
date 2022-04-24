#[macro_use]
extern crate log;

use anyhow::anyhow;
use anyhow::Result;
use pico_args::Arguments;
use simplelog::Config;
use simplelog::LevelFilter;
use simplelog::WriteLogger;
use std::fs::File;
use std::path::PathBuf;
use std::process::Command;

mod ui;

fn main() -> Result<()> {
    let mut args = Arguments::from_env();

    let cli = Cli {
        // exec: args.value_from_str("--exec")?,
        log: args.opt_value_from_str("--log")?,
    };

    // Initialize logging
    let log_path = if let Some(path) = &cli.log {
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
    WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create(log_path)?,
    )?;

    debug!("{:#?}", cli);

    // let _command = Command::new(cli.exec).spawn()?;

    Ok(())
}

#[derive(Debug)]
struct Cli {
    // exec: String,
    log: Option<PathBuf>,
}

// TODO: Help information
