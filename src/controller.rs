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
                AdapterMessage::Event(event) => handle_event(&mut adapter, event.event),
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
            trace!("COMMAND: [{}]", cmd);
            match cmd {
                "in" | "stepin" => {
                    adapter
                        .send_request(Request::StepIn(StepInRequestArgs {
                            thread_id: 1, // TEMPORARY:
                            single_thread: false,
                            target_id: None,
                            granularity: SteppingGranularity::Statement,
                        }))
                        .unwrap();
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
        let mut adapter = adapter.lock().unwrap();
        let adapter_id = adapter.config.adapter_id.clone();

        // Send initialize request
        adapter.send_request(Request::Initialize(InitializeRequestArgs {
            client_id: Some("pesticide".to_string()),
            client_name: Some("Pesticide".to_string()),
            adapter_id,
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
        }))?;
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
        Event::Continued(_) => {
            println!("Continuing?");
        }
        Event::Exited(_) => handle_exited(adapter),
        Event::Output(event) => {
            match event.category {
                Some(OutputEventCategory::Telemetry) => {
                    info!("IDGAF about telemetry")
                } // IDGAF about telemetry
                _ => info!("[DEBUG ADAPTER] >> {}", event.output),
            }
        }
        Event::Initialized => {
            info!("Debug adapter is initialized");
            // TODO: setBreakpoints, etc...
            adapter.send_request(Request::ConfigurationDone).unwrap();
        }
        Event::Process(_) => (), // TODO: What is this event useful for?
        Event::Stopped(event) => {
            println!("STOPPED on thread {}: {:?}", event.thread_id, event.reason);

            // Request threads
            adapter.send_request(Request::Threads).unwrap();
        }
        Event::Thread(event) => {
            info!("New thread started: {}", event.thread_id);
            match event.reason {
                ThreadReason::Started => {
                    adapter.threads.insert(
                        event.thread_id,
                        Thread {
                            id: event.thread_id,
                            // This will be replaced with the actual names in the Threads request
                            name: format!("{}", event.thread_id),
                        },
                    );
                }
                ThreadReason::Exited => {
                    if adapter.threads.remove(&event.thread_id).is_none() {
                        error!("Thread {} ended, but had no stored data", event.thread_id)
                    }
                }
            };
        }
    }
}

fn handle_request(adapter: &mut MutexGuard<Adapter>, payload: RequestPayload) {
    {
        // The only "reverse request" in the DAP is RunInTerminal
        if let Request::RunInTerminal(mut req) = payload.request {
            let mut term_cmd = adapter.config.term_cmd.clone();
            term_cmd.append(&mut req.args);

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

            adapter
                .send_response(
                    payload.seq,
                    success,
                    message,
                    Response::RunInTerminal(RunInTerminalResponseBody {
                        process_id: cmd.ok().map(|child| child.id()),
                        shell_process_id: None, // TEMPORARY:
                    }),
                )
                .unwrap();
        }
    }
}

fn handle_response(adapter: &mut MutexGuard<Adapter>, res: ResponsePayload) {
    match res.response {
        Response::ConfigurationDone => (),
        Response::Initialize(capabilities) => {
            // Save capabilities to Adapter
            adapter.capabilities = Some(capabilities);

            // Send launch request
            // This differs from how the DAP event order is specified on the DAP website
            // See https://github.com/microsoft/vscode/issues/4902#issuecomment-368583522
            let launch_args = adapter.config.launch_args.clone();
            adapter
                .send_request(Request::Launch(LaunchRequestArgs {
                    no_debug: false,
                    restart: None,
                    args: Some(launch_args),
                }))
                .unwrap();
        }
        Response::Launch => {
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
            println!("{:#?}", res);
        }
        Response::StepIn => (),
        Response::Threads(res) => {
            // Update the stored threads
            let threads = &res.threads;
            adapter.threads = threads
                .iter()
                .cloned()
                .map(|thread| (thread.id, thread))
                .collect();

            // Request stack frames for each thread
            for thread in threads {
                adapter
                    .send_request(Request::StackTrace(StackTraceRequestArgs {
                        thread_id: thread.id,
                        start_frame: None,
                        levels: None,
                        format: None,
                    }))
                    .unwrap();
            }
        }
    }
}
