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
use std::collections::VecDeque;
use std::fs::File;
use std::path::PathBuf;
use std::process::{Command, Stdio};
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
                                _ => info!("[DEBUG ADAPTER] >> {}", body.output),
                            }
                        }
                    }
                    Event::Initialized(_) => {
                        info!("Debug adapter is initialized");
                        // TODO: setBreakpoints, etc...
                    }
                },
                AdapterMessage::Request(req) => match req {
                    Request::Initialize(_) => (),
                    Request::Launch(_) => (),
                    Request::RunInTerminal(payload) => {
                        if let Some(args) = payload.args {
                            let mut cmd_args: VecDeque<String> = args.args.into();
                            let cmd = cmd_args
                                .pop_front()
                                .expect("Debug adapter did not provide a command to run");
                            // TEMPORARY: Use the terminal we are currently in as the external terminal
                            let cmd = Command::new(cmd)
                                .args(cmd_args)
                                .stdin(Stdio::piped())
                                .stdout(Stdio::piped())
                                .stderr(Stdio::piped())
                                .spawn();

                            let (success, message) = match &cmd {
                                Ok(_) => (true, None),
                                Err(e) => {
                                    error!("Could not start debugee: {}", e);
                                    (false, Some(e.to_string()))
                                }
                            };

                            let mut adapter = event_adapter.lock().unwrap();
                            let res = AdapterMessage::Response(Response::RunInTerminal(
                                ResponsePayload {
                                    seq: adapter.next_seq(),
                                    request_seq: payload.seq,
                                    success,
                                    message,
                                    body: Some(RunInTerminalResponse {
                                        process_id: cmd.ok().map(|child| child.id()),
                                        shell_process_id: None, // TEMPORARY:
                                    }),
                                },
                            ));

                            adapter.tx.send(res).unwrap();
                        }
                    }
                },
                // TODO: Response state - right now it will fail to deserialize if it did not succeed
                // See https://github.com/serde-rs/serde/pull/2056#issuecomment-1109389651
                AdapterMessage::Response(res) => match res {
                    Response::Initialize(payload) => {
                        // Save capabilities to Adapter
                        let mut adapter = event_adapter.lock().unwrap();
                        adapter.capabilities = payload.body;

                        // Send launch request
                        // This differs from how the DAP event order is specified on the DAP website
                        // See https://github.com/microsoft/vscode/issues/4902#issuecomment-368583522
                        let seq = adapter.next_seq();
                        adapter
                            .tx
                            .send(AdapterMessage::Request(Request::Launch(RequestPayload {
                                args: Some(adapter.config.launch_args.clone()),
                                seq,
                            })))
                            .unwrap();
                    }
                    Response::Launch(payload) => {
                        if payload.success {
                        } else {
                            error!(
                                "Could not launch debug adapter: {}",
                                payload.message.unwrap_or_default()
                            );
                        }
                    } // Unhandled
                    Response::RunInTerminal(_) => (),
                },
            }
        }
    });

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
            supports_run_in_terminal_request: true,
            supports_memory_references: false,
            supports_progress_reporting: false,
            supports_invalidated_event: false,
            supports_memory_event: false,
        }),
        seq: 0,
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
