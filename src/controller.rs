use crate::adapter::Adapter;
use crate::dap_types::*;
use crate::types::*;
use anyhow::Result;
use std::io::Write;
use std::process::Command;
use std::sync::{Arc, Mutex, MutexGuard};
use std::{io, thread};

pub fn start(adapter: Arc<Mutex<Adapter>>) -> Result<()> {
    // Handle incoming messages
    let event_adapter = adapter.clone();
    let event_loop = thread::spawn(move || {
        let rx = event_adapter.lock().unwrap().rx.clone();
        for msg in rx {
            let mut adapter = event_adapter.lock().unwrap();
            match msg {
                AdapterMessage::Event(event) => match event {
                    Event::Exited(_) => handle_exited(&mut adapter),
                    Event::Output(payload) => {
                        trace!("Updating seq");
                        adapter.update_seq(payload.seq);
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
                                seq: adapter.next_seq(),
                                args: None,
                            }));

                        adapter.tx.send(req).unwrap();
                    }
                    Event::Process(_) => (), // TODO: What is this event useful for?
                    Event::Stopped(event) => {
                        if let Some(body) = event.body {
                            info!("STOPPED on thread {}", body.thread_id);
                        }
                    }
                    Event::Thread(event) => {
                        if let Some(body) = event.body {
                            info!("New thread started: {}", body.thread_id);
                            match body.reason {
                                ThreadReason::Started => {
                                    adapter.threads.insert(body.thread_id, Thread {});
                                }
                                ThreadReason::Exited => {
                                    if adapter.threads.remove(&body.thread_id).is_none() {
                                        error!(
                                            "Thread {} ended, but had no stored data",
                                            body.thread_id
                                        )
                                    }
                                }
                            };
                        }
                    }
                },
                AdapterMessage::Request(req) => {
                    if let Request::RunInTerminal(req) = req {
                        if let Some(mut args) = req.args {
                            let mut term_cmd = adapter.config.term_cmd.clone();
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

                            let mut adapter = adapter;
                            let res = AdapterMessage::Response(Response::RunInTerminal(
                                ResponsePayload {
                                    seq: adapter.next_seq(),
                                    request_seq: req.seq,
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
                    Response::Initialize(res) => {
                        // Save capabilities to Adapter
                        let mut adapter = adapter;
                        adapter.capabilities = res.body;

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
                    Response::Launch(res) => {
                        if res.success {
                        } else {
                            error!(
                                "Could not launch debug adapter: {}",
                                res.message.unwrap_or_default()
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
        let stdout = std::io::stdout();
        loop {
            print!("> ");
            stdout.lock().flush().unwrap();
            let mut cmd = String::new();
            stdin.read_line(&mut cmd).expect("Failed to read stdin");

            let mut adapter = cli_adapter.lock().unwrap();

            let cmd = cmd.trim();
            match cmd {
                "exit" => {
                    handle_exited(&mut adapter);
                    return;
                }
                _ => eprintln!("Unrecognized command: '{}'", cmd),
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

fn handle_exited(adapter: &mut MutexGuard<Adapter>) {
    // Stop the adapter
    if let Err(e) = adapter.child.kill() {
        error!("Failed to kill debug adapter: {}", e)
    }
    // Pesticide will exit due to the debug adapter pipe closing
}
