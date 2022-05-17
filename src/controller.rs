use crate::adapter::Adapter;
use crate::config::Config;
use crate::dap_types::*;
use anyhow::{anyhow, bail, Result};
use futures_util::StreamExt;
use itertools::Itertools;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::select;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::codec::{FramedRead, LinesCodec};

pub async fn run(config_path: PathBuf) -> Result<()> {
    // Parse configuration
    // Do this first so we can display the error in the terminal
    let config = Config::new(config_path)?;

    // Initialize state
    let mut state = State::new();
    // Initialize UI
    let mut ui = crate::ui::Ui::new().await?;
    // Draw with initial state
    ui.draw(&state)?;

    // Channel to read debugee stdout through
    let (debugee_tx, mut debugee_rx) = tokio::sync::mpsc::unbounded_channel();

    // Spin up debug adapter
    let mut adapter = Adapter::new(config)?;
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

    'main: loop {
        // Act on incoming messages
        let mut actions = vec![];
        select! {
            // Incoming debug adapter messages
            res = adapter.read() => {
                match res {
                    Ok(Some(msg)) => {
                        actions.append(&mut match msg {
                            AdapterMessage::Event(payload) => handle_event(&mut state, &mut adapter, payload).await?,
                            AdapterMessage::Request(payload) => handle_request(&mut adapter, payload, debugee_tx.clone()).await?,
                            AdapterMessage::Response(payload) => handle_response(&mut state, &mut adapter, payload).await?,
                        });
                    },
                    Ok(None) => {
                        info!("Debug adapter shut down, ending session");
                        break
                    },
                    Err(e) => error!("{}", e)
                }
            }
            // Debugee stdout
            Some(line) = debugee_rx.recv() => {
                state.console.push(line);
                actions.push(Action::Redraw);
            }
            // User input
            Some(Ok(event)) = ui.input_stream.next() => {
                actions.append(&mut ui.handle_input(&mut state, event)?)
            }
        }

        // Dispatch needed actions
        for action in actions {
            match action {
                Action::Redraw => ui.draw(&state)?,
                Action::Request(req) => {
                    adapter.send_request(req).await?;
                }
                Action::Quit => break 'main,
            };
        }
    }

    trace!("Cleaning up");
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

    pub console: Vec<String>,

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

            console: vec![],

            threads: vec![],
            stack_frames: HashMap::new(),
            scopes: HashMap::new(),
            variables: HashMap::new(),
        }
    }
}

pub enum Action {
    Quit,
    Redraw,
    Request(Request),
}

async fn handle_event(
    state: &mut State,
    adapter: &mut Adapter,
    payload: EventPayload,
) -> Result<Vec<Action>> {
    adapter.update_seq(payload.seq);

    let mut actions = vec![];
    match payload.event {
        Event::Continued(event) => {
            if event.all_threads_continued {
                state.stopped_threads.clear();
            } else {
                state.stopped_threads.remove(&event.thread_id);
            }
        }
        Event::Exited(_) => {
            actions.push(Action::Quit);
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

            actions.push(Action::Redraw);
        }
    };

    Ok(actions)
}

async fn handle_request(
    adapter: &mut Adapter,
    payload: RequestPayload,
    debugee_tx: UnboundedSender<String>,
) -> Result<Vec<Action>> {
    adapter.update_seq(payload.seq);

    // The only "reverse request" in the DAP is RunInTerminal
    if let Request::RunInTerminal(mut req) = payload.request {
        debug!("{:?}", req.args);
        let mut child = match req.kind {
            RunInTerminalKind::External => {
                let mut cmd = adapter.config.term_cmd.clone();
                cmd.append(&mut req.args);
                Command::new(cmd[0].clone()).args(cmd[1..].to_vec()).spawn()
            }
            // SAFETY: We are simply calling setsid(), which is a libc function
            RunInTerminalKind::Integrated => unsafe {
                Command::new(req.args[0].clone())
                    .args(req.args[1..].to_vec())
                    .stdin(Stdio::null())
                    .stderr(Stdio::null())
                    .stdout(Stdio::piped())
                    .pre_exec(|| {
                        let pid = libc::setsid();
                        if pid == -1 {
                            // FIXME: This is awful
                            panic!("Failed call to setsid() for debugee");
                        }
                        Ok(())
                    })
                    .spawn()
            },
        }?;

        adapter
            .send_response(
                payload.seq,
                true,
                None,
                Response::RunInTerminal(RunInTerminalResponse {
                    process_id: child.id(),
                    shell_process_id: None,
                }),
            )
            .await?;

        if let RunInTerminalKind::Integrated = req.kind {
            // Send stdout to main loop
            let mut stdout = FramedRead::new(
                child
                    .stdout
                    .take()
                    .ok_or_else(|| anyhow!("Failed to take debugee stdout"))?,
                LinesCodec::new(),
            );
            tokio::spawn(async move {
                while let Some(Ok(line)) = stdout.next().await {
                    debugee_tx.send(line).unwrap();
                }
            });
        }
    };

    Ok(vec![])
}

async fn handle_response(
    state: &mut State,
    adapter: &mut Adapter,
    payload: ResponsePayload,
) -> Result<Vec<Action>> {
    adapter.update_seq(payload.seq);

    // Get the request that triggered this response
    let req = adapter.get_request(payload.request_seq);

    let mut actions = vec![];

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
                // Request scopes for current stack frame
                if res
                    .stack_frames
                    .iter()
                    .any(|frame| frame.id == state.current_stack_frame)
                {
                    adapter
                        .send_request(Request::Scopes(ScopesArgs {
                            frame_id: state.current_stack_frame,
                        }))
                        .await?;
                }
                // Add to state
                state.stack_frames.insert(req.thread_id, res.stack_frames);
            }
        }
        Response::StepIn => (),
        Response::Threads(res) => {
            // Update the stored threads
            let threads = &res.threads;
            state.threads = threads.clone();

            // Request stack frames for the current thread
            if state
                .threads
                .iter()
                .any(|thread| thread.id == state.current_thread)
            {
                adapter
                    .send_request(Request::StackTrace(StackTraceArgs {
                        thread_id: state.current_thread,
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
                // Set current stack frame to topmost of current thread.
                // If the current thread has no stack frames, or they're
                // somehow empty, select none of them.
                state.current_stack_frame = state.stack_frames[&state.current_thread]
                    .first()
                    .map(|frame| frame.id)
                    .unwrap_or(0);
            }
        }
    };

    if adapter.num_requests() == 0 {
        actions.push(Action::Redraw);
    }

    Ok(actions)
}
