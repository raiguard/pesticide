use crate::adapter::Adapter;
use crate::{config, dap_types::*};
use anyhow::Result;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::{io, thread};

pub fn start(adapter: Arc<Mutex<Adapter>>) -> Result<()> {
    // Handle incoming messages
    let event_adapter = adapter.clone();
    let event_loop = thread::spawn(move || {
        let rx = event_adapter.lock().unwrap().rx.clone();
        for msg in rx {
            match msg {
                AdapterMessage::Event(event) => match event {
                    Event::Exited(_) => handle_exited(&event_adapter),
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
                        let req =
                            AdapterMessage::Request(Request::ConfigurationDone(RequestPayload {
                                seq: event_adapter.lock().unwrap().next_seq(),
                                args: None,
                            }));

                        event_adapter.lock().unwrap().tx.send(req).unwrap();
                    }
                },
                AdapterMessage::Request(req) => {
                    if let Request::RunInTerminal(payload) = req {
                        if let Some(mut args) = payload.args {
                            let mut term_cmd =
                                event_adapter.lock().unwrap().config.term_cmd.clone();
                            term_cmd.append(&mut args.args);

                            let cmd = Command::new(term_cmd[0].clone())
                                .args(term_cmd[1..].to_vec())
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
                }
                AdapterMessage::Response(res) => match res {
                    Response::ConfigurationDone(_) => (),
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
                    }
                    Response::RunInTerminal(_) => (),
                },
            }
        }
    });

    // Basic CLI
    let cli_adapter = adapter.clone();
    let cli_loop = thread::spawn(move || {
        let stdin = io::stdin();
        loop {
            let mut cmd = String::new();
            stdin.read_line(&mut cmd).expect("Failed to read stdin");

            match cmd.trim() {
                "exit" => {
                    handle_exited(&cli_adapter);
                    return;
                }
                _ => error!("Unrecognized command: '{}'", cmd),
            }
        }
    });

    {
        let adapter = adapter.lock().unwrap();

        // Send initialize request
        let init = AdapterMessage::Request(Request::Initialize(RequestPayload {
            args: Some(InitializeRequest {
                client_id: Some("pesticide".to_string()),
                client_name: Some("Pesticide".to_string()),
                adapter_id: adapter.config.adapter_id.clone(),
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
        adapter.tx.send(init)?;
    }

    event_loop.join().unwrap();
    cli_loop.join().unwrap();

    Ok(())
}

fn handle_exited(adapter: &Arc<Mutex<Adapter>>) {
    // Stop the adapter
    if let Err(e) = adapter.lock().unwrap().child.kill() {
        error!("Failed to kill debug adapter: {}", e)
    }
    // Pesticide will exit due to the debug adapter pipe closing
}
