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
use std::sync::{Arc, Mutex};
use std::thread;

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
    let adapter = Arc::new(Mutex::new(Adapter::new(config.clone())?));

    // Handle incoming messages
    let event_rx = adapter.lock().unwrap().rx.clone();
    let event_adapter = adapter.clone();
    let event_loop = thread::spawn(move || {
        for msg in event_rx {
            match msg {
                AdapterMessage::Event(event) => match event {
                    Event::Output(payload) => {
                        trace!("Updating seq");
                        event_adapter.lock().unwrap().update_seq(payload.seq);
                        trace!("Updated seq");
                        if let Some(body) = payload.body {
                            match body.category {
                                Some(OutputEventCategory::Telemetry) => {
                                    info!("IDGAF about telemetry")
                                } // IDGAF about telemetry
                                _ => info!("Debug adapter message: {}", body.output),
                            }
                        }
                    }
                },
                AdapterMessage::Request(req) => debug!("RECEIVED REQUEST: {:#?}", req),
                AdapterMessage::Response(res) => match res {
                    Response::Initialize(payload) => {
                        if payload.success {
                            info!("Debug adapter successfully initialized");
                        } else {
                            error!("Debug adapter did not successfully initialize");
                        }
                    }
                },
            }
        }
    });

    thread::sleep(std::time::Duration::from_millis(100));

    // Send initialize request
    let init = AdapterMessage::Request(Request::Initialize(RequestPayload {
        args: Some(InitializeRequest {
            client_id: Some("pesticide".to_string()),
            client_name: Some("Pesticide".to_string()),
            adapter_id: config.adapter_id,
            lines_start_at_1: true,
            columns_start_at_1: true,
            path_format: Some(InitializeRequestPathFormat::Path),
            supports_variable_type: false,
            supports_variable_paging: false,
            supports_run_in_terminal_request: false,
            supports_memory_references: false,
            supports_progress_reporting: false,
            supports_invalidated_event: false,
            supports_memory_event: false,
        }),
        seq: adapter.lock().unwrap().next_seq(),
    }));

    adapter.lock().unwrap().tx.send(init)?;

    event_loop.join().unwrap();

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
