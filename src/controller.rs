use crate::adapter::Adapter;
use crate::config::Config;
use crate::dap_types::*;
use anyhow::Result;
use futures_util::StreamExt;
use itertools::Itertools;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::select;

pub async fn run(config_path: PathBuf) -> Result<()> {
    // Initialize state
    let mut state = State::new();
    // Initialize UI
    let mut ui = crate::ui::Ui::new().await?;
    // Draw with initial state
    ui.draw(&state)?;

    // Spin up debug adapter
    let mut adapter = Adapter::new(Config::new(config_path)?)?;
    // Send initialize request
    let adapter_id = adapter.config.adapter_id.clone();
    adapter
        .send_request(Request::Initialize(InitializeArgs {
            client_id: Some("pesticide".to_string()),
            client_name: Some("Pesticide".to_string()),
            adapter_id,
            locale: Some("en-US".to_string()),
            lines_start_at_1: true,
            columns_start_at_1: true,
            path_format: Some(InitializePathFormat::Path),
            supports_variable_type: false,
            supports_variable_paging: false,
            supports_run_in_terminal_request: true,
            supports_memory_references: false,
            supports_progress_reporting: false,
            supports_invalidated_event: false,
            supports_memory_event: false,
        }))
        .await?;

    loop {
        // Act on incoming messages
        let mut action = None;
        select! {
            // Incoming debug adapter messages
            res = adapter.read() => {
                match res {
                    Ok(Some(msg)) => {
                        action = match msg {
                            AdapterMessage::Event(payload) => handle_event(&mut state, &mut adapter, payload).await?,
                            AdapterMessage::Request(payload) => handle_request(&mut adapter, payload).await?,
                            AdapterMessage::Response(payload) => handle_response(&mut state, &mut adapter, payload).await?,
                        };
                    },
                    Ok(None) => {
                        info!("Debug adapter shut down, ending session");
                        break
                    },
                    Err(e) => error!("{}", e)
                }
            }
            // User input
            Some(Ok(event)) = ui.input_stream.next() => {
                action = ui.handle_input(event)?
            }
            // TODO: Read stdout of child process to show in UI
        }

        // Dispatch needed actions
        if let Some(order) = action {
            match order {
                Action::Redraw => ui.draw(&state)?,
                Action::StepIn => {
                    adapter
                        .send_request(Request::StepIn(StepInArgs {
                            thread_id: state.current_thread,
                            single_thread: true,
                            target_id: None,
                            granularity: SteppingGranularity::Line,
                        }))
                        .await?;
                }
                Action::Quit => break,
                _ => (),
            };
        }
    }

    error!("HOW DID WE GET HERE");
    ui.destroy()?;
    adapter.quit().await?;

    Ok(())
}

pub struct State {
    /// The thread we are currently stopped on
    pub current_thread: u32,
    /// The stack frame we are currently stopped on
    pub current_stack_frame: u32,
    /// Known stopped threads
    /// Any threads that were stopped that we didn't get explicitly will be marked as "paused"
    pub stopped_threads: HashMap<u32, StoppedReason>,
    pub all_threads_stopped: bool,

    pub threads: Vec<Thread>,
    pub stack_frames: HashMap<u32, Vec<StackFrame>>,
    pub scopes: HashMap<u32, Vec<Scope>>,
    pub variables: HashMap<u32, Vec<Variable>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            current_thread: 0,
            current_stack_frame: 0,
            stopped_threads: HashMap::new(),
            all_threads_stopped: false,

            threads: vec![],
            stack_frames: HashMap::new(),
            scopes: HashMap::new(),
            variables: HashMap::new(),
        }
    }
}

pub enum Action {
    Continue,
    Quit,
    Redraw,
    StepIn,
}

async fn handle_event(
    state: &mut State,
    adapter: &mut Adapter,
    payload: EventPayload,
) -> Result<Option<Action>> {
    adapter.update_seq(payload.seq);

    match payload.event {
        Event::Continued(event) => {
            if event.all_threads_continued {
                state.stopped_threads.clear();
            } else {
                state.stopped_threads.remove(&event.thread_id);
            }
        }
        Event::Exited(_) => {
            return Ok(Some(Action::Quit));
        }
        Event::Module(_) => (), // TODO:
        Event::Output(event) => match event.category {
            Some(OutputCategory::Telemetry) => (), // IDGAF about telemetry
            _ => info!("[DEBUG ADAPTER] >> {}", event.output),
        },
        Event::Initialized => {
            info!("Debug adapter is initialized");
            // TODO: setBreakpoints, etc...
            adapter.send_request(Request::ConfigurationDone).await?;
        }
        Event::Process(_) => (), // TODO:
        Event::Stopped(event) => {
            info!("STOPPED on thread {}: {:?}", event.thread_id, event.reason);

            state.current_thread = event.thread_id;
            state.stopped_threads.insert(event.thread_id, event.reason);
            state.all_threads_stopped = event.all_threads_stopped;

            // Request threads.
            // This sets off a chain reaction of events to get all of the info
            // we need.
            adapter.send_request(Request::Threads).await?;
        }
        Event::Thread(event) => {
            info!("New thread started: {}", event.thread_id);
            match event.reason {
                ThreadReason::Started => {
                    state.threads.push(Thread {
                        id: event.thread_id,
                        // This will be replaced with the actual names in the Threads request
                        name: format!("{}", event.thread_id),
                    });
                }
                ThreadReason::Exited => {
                    if let Some((i, _)) = state
                        .threads
                        .iter()
                        .find_position(|thread| thread.id == event.thread_id)
                    {
                        state.threads.remove(i);
                        state.stopped_threads.remove(&(i as u32));
                    }
                }
            };

            return Ok(Some(Action::Redraw));
        }
    };

    Ok(None)
}

async fn handle_request(adapter: &mut Adapter, payload: RequestPayload) -> Result<Option<Action>> {
    adapter.update_seq(payload.seq);

    // The only "reverse request" in the DAP is RunInTerminal
    if let Request::RunInTerminal(mut req) = payload.request {
        let cmd = match req.kind {
            RunInTerminalKind::External => {
                let mut term_cmd = adapter.config.term_cmd.clone();
                term_cmd.append(&mut req.args);
                term_cmd
            }
            RunInTerminalKind::Integrated => req.args,
        };

        let cmd = Command::new(cmd[0].clone())
            .args(cmd[1..].to_vec())
            .stdin(Stdio::null()) // So we can still ctrl+c the server
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
                Response::RunInTerminal(RunInTerminalResponse {
                    process_id: cmd.ok().and_then(|child| child.id()),
                    shell_process_id: None, // TEMPORARY:
                }),
            )
            .await?;
    };

    Ok(None)
}

async fn handle_response(
    state: &mut State,
    adapter: &mut Adapter,
    payload: ResponsePayload,
) -> Result<Option<Action>> {
    adapter.update_seq(payload.seq);

    // Get the request that triggered this response
    let req = adapter.get_request(payload.request_seq);

    match payload.response {
        Response::ConfigurationDone => (),
        Response::Continue(_) => (),
        Response::Initialize(capabilities) => {
            // Save capabilities to Adapter
            adapter.capabilities = Some(capabilities);

            // Send launch request
            // This differs from how the DAP event order is specified on the DAP website
            // See https://github.com/microsoft/vscode/issues/4902#issuecomment-368583522
            let launch_args = adapter.config.launch_args.clone();
            adapter
                .send_request(Request::Launch(LaunchArgs {
                    no_debug: false,
                    restart: None,
                    args: Some(launch_args),
                }))
                .await?;
        }
        Response::Launch => {
            if payload.success {
            } else {
                error!(
                    "Could not launch debug adapter: {}",
                    payload.message.unwrap_or_default()
                );
            }
        }
        Response::RunInTerminal(_) => (),
        Response::Scopes(res) => {
            if let Some(Request::Scopes(req)) = req {
                for scope in &res.scopes {
                    adapter
                        .send_request(Request::Variables(VariablesArgs {
                            variables_reference: scope.variables_reference,
                            filter: None,
                            start: None,
                            count: None,
                            format: None,
                        }))
                        .await?;
                }

                state.scopes.insert(req.frame_id, res.scopes);
            }
        }
        Response::StackTrace(res) => {
            if let Some(Request::StackTrace(req)) = req {
                for stack_frame in &res.stack_frames {
                    adapter
                        .send_request(Request::Scopes(ScopesArgs {
                            frame_id: stack_frame.id,
                        }))
                        .await?;
                }

                state.current_stack_frame = res.stack_frames.get(0).unwrap().id;
                state.stack_frames.insert(req.thread_id, res.stack_frames);
            }
        }
        Response::StepIn => (),
        Response::Threads(res) => {
            // Update the stored threads
            let threads = &res.threads;
            state.threads = threads.clone();

            // Request stack frames for each thread
            for thread in threads {
                adapter
                    .send_request(Request::StackTrace(StackTraceArgs {
                        thread_id: thread.id,
                        start_frame: None,
                        levels: None,
                        format: None,
                    }))
                    .await?;
            }
        }
        Response::Variables(res) => {
            if let Some(Request::Variables(req)) = req {
                state
                    .variables
                    .insert(req.variables_reference, res.variables);
            }

            if adapter.num_requests() == 0 {
                return Ok(Some(Action::Redraw));
            }
        }
    };

    Ok(None)
}
