use crate::adapter::Adapter;
use crate::dap_types::*;
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
                AdapterMessage::Event(event) => handle_event(&mut adapter, event),
                AdapterMessage::Request(req) => handle_request(&mut adapter, req),
                AdapterMessage::Response(res) => handle_response(&mut adapter, res),
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
                "in" | "stepin" => {
                    let req = AdapterMessage::Request(Request::StepIn(RequestPayload {
                        seq: adapter.next_seq(),
                        args: Some(StepInRequest {
                            thread_id: 1, // TEMPORARY:
                            single_thread: false,
                            target_id: None,
                            granularity: SteppingGranularity::Statement,
                        }),
                    }));

                    adapter.tx.send(req).unwrap();
                }
                "threads" => {
                    for thread in adapter.threads.values() {
                        println!("{}", thread.name);
                    }
                }
                "quit" | "q" => {
                    handle_exited(&mut adapter);
                    return;
                }
                _ => eprintln!("Unrecognized command: '{}'", cmd),
            }
        }
    });

    {
        // This will be dropped at the end of this inner scope, freeing it up
        let adapter = adapter.lock().unwrap();

        // Send initialize request
        let init = AdapterMessage::Request(Request::Initialize(RequestPayload {
            args: Some(InitializeRequest {
                client_id: Some("pesticide".to_string()),
                client_name: Some("Pesticide".to_string()),
                adapter_id: adapter.config.adapter_id.clone(),
                locale: Some("en-US".to_string()),
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

fn handle_event(adapter: &mut MutexGuard<Adapter>, event: Event) {
    match event {
        Event::Exited(_) => handle_exited(adapter),
        Event::Output(payload) => {
            adapter.update_seq(payload.seq);
            if let Some(body) = payload.body {
                match body.category {
                    Some(OutputEventCategory::Telemetry) => {
                        info!("IDGAF about telemetry")
                    } // IDGAF about telemetry
                    _ => info!("[DEBUG ADAPTER] >> {}", body.output),
                }
            }
        }
        Event::Initialized(payload) => {
            adapter.update_seq(payload.seq);
            info!("Debug adapter is initialized");
            // TODO: setBreakpoints, etc...
            let req = AdapterMessage::Request(Request::ConfigurationDone(RequestPayload {
                seq: adapter.next_seq(),
                args: None,
            }));

            adapter.tx.send(req).unwrap();
        }
        Event::Process(payload) => adapter.update_seq(payload.seq), // TODO: What is this event useful for?
        Event::Stopped(payload) => {
            adapter.update_seq(payload.seq);
            if let Some(body) = payload.body {
                println!("STOPPED on thread {}: {:?}", body.thread_id, body.reason);

                // Request threads
                let req = AdapterMessage::Request(Request::Threads(RequestPayload {
                    seq: adapter.next_seq(),
                    args: None,
                }));
                adapter.tx.send(req).unwrap();
            }
        }
        Event::Thread(payload) => {
            adapter.update_seq(payload.seq);
            if let Some(body) = payload.body {
                info!("New thread started: {}", body.thread_id);
                match body.reason {
                    ThreadReason::Started => {
                        adapter.threads.insert(
                            body.thread_id,
                            Thread {
                                id: body.thread_id,
                                // This will be replaced with the actual names in the Threads request
                                name: format!("{}", body.thread_id),
                            },
                        );
                    }
                    ThreadReason::Exited => {
                        if adapter.threads.remove(&body.thread_id).is_none() {
                            error!("Thread {} ended, but had no stored data", body.thread_id)
                        }
                    }
                };
            }
        }
    }
}

fn handle_request(adapter: &mut MutexGuard<Adapter>, req: Request) {
    {
        // The only "reverse request" in the DAP is RunInTerminal
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

                let res = AdapterMessage::Response(Response::RunInTerminal(ResponsePayload {
                    seq: adapter.next_seq(),
                    request_seq: req.seq,
                    success,
                    message,
                    body: Some(RunInTerminalResponse {
                        process_id: cmd.ok().map(|child| child.id()),
                        shell_process_id: None, // TEMPORARY:
                    }),
                }));

                adapter.tx.send(res).unwrap();
            }
        }
    }
}

fn handle_response(adapter: &mut MutexGuard<Adapter>, res: Response) {
    match res {
        Response::ConfigurationDone(_) => (),
        Response::Initialize(res) => {
            // Save capabilities to Adapter
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
        Response::StackTrace(res) => {
            if let Some(body) = res.body {
                println!("{:#?}", body);
            }
        }
        Response::StepIn(_) => (),
        Response::Threads(res) => {
            if let Some(body) = res.body {
                // Update the stored threads
                let threads = &body.threads;
                adapter.threads = threads
                    .iter()
                    .cloned()
                    .map(|thread| (thread.id, thread))
                    .collect();

                // Request stack frames for each thread
                for thread in threads {
                    let req = AdapterMessage::Request(Request::StackTrace(RequestPayload {
                        seq: adapter.next_seq(),
                        args: Some(StackTraceRequest {
                            thread_id: thread.id,
                            start_frame: None,
                            levels: None,
                            format: None,
                        }),
                    }));

                    adapter.tx.send(req).unwrap();
                }
            }
        }
    }
}
