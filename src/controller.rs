use crate::adapter::Adapter;
use crate::config::Config;
use crate::dap::*;
use crate::kakoune::{KakRequest, Kakoune};
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

pub async fn run(config_path: PathBuf, sock_path: PathBuf, session: String) -> Result<()> {
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

    // Kakoune comms
    let mut kakoune = Kakoune::new(session, sock_path).await?;

    // Spin up debug adapter
    let mut adapter = Adapter::new(config)?;
    // Send initialize request
    let adapter_id = adapter.config.adapter_id.clone();
    adapter
        .send_request(RequestArguments::initialize(InitializeRequestArguments {
            client_id: Some("pesticide".to_string()),
            client_name: Some("Pesticide".to_string()),
            adapter_id: adapter_id.unwrap(),
            locale: Some("en-US".to_string()),
            lines_start_at_1: Some(true),
            columns_start_at_1: Some(true),
            path_format: Some(String::from("path")),
            supports_variable_type: None,
            supports_variable_paging: None,
            supports_run_in_terminal_request: Some(true),
            supports_memory_references: None,
            supports_progress_reporting: None,
            supports_invalidated_event: None,
            supports_memory_event: None,
        }))
        .await?;

    'main: loop {
        // Act on incoming messages
        let mut actions = vec![];
        select! {
            // Incoming debug adapter messages
            res = adapter.stdout.next() => {
                match res {
                    Some(res) => {
                        match res {
                            Ok(Ok(msg)) => {
                                adapter.update_seq(msg.seq);
                                actions.append(&mut match msg.type_ {
                                    ProtocolMessageType::Event(event) => handle_event(
                                        &mut state,
                                        &mut adapter,
                                        event
                                    ).await?,
                                    ProtocolMessageType::Request(request) => handle_request(
                                        &mut adapter,
                                        request,
                                        msg.seq,
                                        debugee_tx.clone()
                                    ).await?,
                                    ProtocolMessageType::Response(response) => handle_response(
                                        &mut state,
                                        &mut adapter,
                                        response
                                    ).await?,
                                });
                            },
                            Ok(Err(e)) => {
                                error!("{:?}", e);
                            },
                            Err(e) => error!("{}", e)
                        }
                    }
                    None => {
                        info!("Debug adapter exited, shutting down");
                        break
                    }
                }
            }
            // Debugee stdout
            Some(line) = debugee_rx.recv() => {
                trace!("debugee: {}", line);
                state.console.push(line);
                actions.push(Action::Redraw);
            }
            // User input
            Some(Ok(event)) = ui.input_stream.next() => {
                actions.append(&mut ui.handle_input(&mut state, event)?)
            }
            // Requests from Kakoune
            Ok(req) = kakoune.recv() => {
                match req {
                    KakRequest::ToggleBreakpoint { file, line, column: _column } => {
                        let source_breakpoints = state.breakpoints.entry(file.clone()).or_default();
                        if let Some((i, _)) = source_breakpoints.iter().find_position(|breakpoint| breakpoint.line == line) {
                            source_breakpoints.remove(i);
                        } else {
                            source_breakpoints.push(SourceBreakpoint {
                                column: None,
                                condition: None,
                                hit_condition: None,
                                line,
                                log_message: None
                            });
                        };
                        let req = SetBreakpointsArguments {
                            breakpoints: Some(source_breakpoints.clone()),
                            lines: None,
                            source: Source {
                                adapter_data: None,
                                checksums: None,
                                name: None,
                                origin: None,
                                path:Some(file),
                                presentation_hint: None,
                                source_reference: None,
                                sources: None
                            },
                            source_modified: None
                        };
                        actions.push(Action::Request(RequestArguments::setBreakpoints(req)));
                        actions.push(Action::UpdateBreakpoints);
                    },
                }
            }
        }

        // Dispatch needed actions
        for action in actions {
            match action {
                Action::ClearJump => kakoune.clear_jump().await?,
                Action::Jump => kakoune.jump(&state).await?,
                Action::Quit => break 'main,
                Action::Redraw => ui.draw(&state)?,
                Action::Request(req) => {
                    adapter.send_request(req).await?;
                }
                Action::UpdateBreakpoints => kakoune.update_breakpoints(&state).await?,
            };
        }
    }

    trace!("Cleaning up");
    ui.destroy()?;
    adapter.quit().await?;
    kakoune.quit().await?;
    trace!("Cleaned up");

    Ok(())
}

pub struct State {
    /// The thread we are currently stopped on
    pub current_thread: i64,
    /// The stack frame we are currently stopped on
    pub current_stack_frame: i64,
    /// Known stopped threads
    /// Any threads that were stopped that we didn't get explicitly will be marked as "paused"
    pub stopped_threads: HashMap<i64, String>,
    pub all_threads_stopped: bool,

    pub console: Vec<String>,

    pub threads: Vec<Thread>,
    // Thread ID -> Stack frames
    pub stack_frames: HashMap<i64, Vec<StackFrame>>,
    pub scopes: HashMap<i64, Vec<Scope>>,
    pub variables: HashMap<i64, Vec<Variable>>,

    pub breakpoints: HashMap<String, Vec<SourceBreakpoint>>,
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

            breakpoints: HashMap::new(),
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum Action {
    ClearJump,
    Jump,
    Quit,
    Redraw,
    Request(RequestArguments),
    UpdateBreakpoints,
}

async fn handle_event(
    state: &mut State,
    adapter: &mut Adapter,
    event: EventBody,
) -> Result<Vec<Action>> {
    let mut actions = vec![];
    match event {
        EventBody::continued(event) => {
            if event.all_threads_continued.unwrap_or_default() {
                state.stopped_threads.clear();
            } else {
                state.stopped_threads.remove(&event.thread_id);
            }
        }
        EventBody::exited(_) | EventBody::terminated(_) => {
            actions.push(Action::Quit);
        }
        EventBody::module(_) => (), // TODO:
        EventBody::output(event) => match event.category.as_deref() {
            Some("telemetry") => (), // IDGAF about telemetry
            _ => info!("Adapter output event: {}", event.output),
        },
        EventBody::initialized => {
            info!("Debug adapter is initialized");
            // TODO: setBreakpoints, etc...
            adapter
                .send_request(RequestArguments::configurationDone(None))
                .await?;
        }
        EventBody::stopped(event) => {
            // TODO: Is there ever a time where this will be None?
            let thread_id = event.thread_id.unwrap();
            info!(
                "[{}] STOPPED [{:?}]",
                thread_id,
                event.description.as_ref().unwrap_or(&event.reason)
            );

            state.current_thread = thread_id;
            state.stopped_threads.insert(thread_id, event.reason);
            state.all_threads_stopped = event.all_threads_stopped.unwrap_or_default();

            // Request threads.
            // This sets off a chain reaction of events to get all of the info
            // we need.
            adapter
                .send_request(RequestArguments::threads(None))
                .await?;
        }
        EventBody::thread(event) => {
            info!("[{}] NEW", event.thread_id);
            match event.reason.as_str() {
                "started" => {
                    state.threads.push(Thread {
                        id: event.thread_id,
                        // This will be replaced with the actual names in the Threads request
                        name: format!("{}", event.thread_id),
                    });
                }
                "exited" => {
                    if let Some((i, _)) = state
                        .threads
                        .iter()
                        .find_position(|thread| thread.id == event.thread_id)
                    {
                        state.threads.remove(i);
                        state.stopped_threads.remove(&(i as i64));
                    }
                }
                reason => error!("Unhandled thread reason: '{reason}"),
            };

            actions.push(Action::Redraw);
        }
        EventBody::breakpoint(_) => (),
        EventBody::capabilities(_) => (),
        EventBody::invalidated(_) => (),
    };

    Ok(actions)
}

async fn handle_request(
    adapter: &mut Adapter,
    request: RequestArguments,
    seq: u32,
    debugee_tx: UnboundedSender<String>,
) -> Result<Vec<Action>> {
    // The only "reverse request" in the DAP is RunInTerminal
    if let RequestArguments::runInTerminal(mut request) = request {
        let mut child = match request.kind.as_deref() {
            Some("external") => {
                let mut cmd = adapter.config.term_cmd.clone();
                cmd.append(&mut request.args);
                Command::new(cmd[0].clone()).args(cmd[1..].to_vec()).spawn()
            }
            // SAFETY: We are simply calling setsid(), which is a libc function
            Some("integrated") | None => unsafe {
                Command::new(request.args[0].clone())
                    .args(request.args[1..].to_vec())
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
            Some(kind) => {
                bail!("Unable to start debugee: unrecognized runInTerminal kind '{kind}'")
            }
        }?;

        adapter
            .send_response(
                seq,
                true,
                ResponseBody::runInTerminal(RunInTerminalResponseBody {
                    process_id: child.id().map(|id| id as i64),
                    shell_process_id: None,
                }),
            )
            .await?;

        if let Some("integrated") | None = request.kind.as_deref() {
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
    response: Response,
) -> Result<Vec<Action>> {
    // Get the request that triggered this response
    let req = adapter.get_request(response.request_seq);

    let mut actions = vec![];

    match response.result {
        ResponseResult::Success { body } => {
            match body {
                ResponseBody::configurationDone => (),
                ResponseBody::continue_(_) => actions.push(Action::ClearJump),
                ResponseBody::initialize(capabilities) => {
                    // Save capabilities to Adapter
                    adapter.capabilities = Some(capabilities);

                    // Send launch request
                    // This differs from how the DAP event order is specified on the DAP website
                    // See https://github.com/microsoft/vscode/issues/4902#issuecomment-368583522
                    let launch_args = adapter.config.launch_args.clone();
                    adapter
                        .send_request(RequestArguments::launch(Either::Second(launch_args)))
                        .await?;
                }
                ResponseBody::launch => (),
                ResponseBody::runInTerminal(_) => (),
                ResponseBody::scopes(res) => {
                    if let Some(RequestArguments::scopes(req)) = req {
                        for scope in &res.scopes {
                            adapter
                                .send_request(RequestArguments::variables(VariablesArguments {
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
                ResponseBody::stackTrace(res) => {
                    if let Some(RequestArguments::stackTrace(req)) = req {
                        if req.thread_id == state.current_thread {
                            // Set current stack frame to topmost of this thread
                            state.current_stack_frame = res
                                .stack_frames
                                .first()
                                .map(|frame| frame.id)
                                .unwrap_or_default();
                            // Request scopes for current stack frame
                            adapter
                                .send_request(RequestArguments::scopes(ScopesArguments {
                                    frame_id: state.current_stack_frame,
                                }))
                                .await?;
                        }
                        // Add to state
                        state.stack_frames.insert(req.thread_id, res.stack_frames);
                        // Jump in editor
                        actions.push(Action::Jump);
                    }
                }
                ResponseBody::stepIn => (),
                ResponseBody::threads(res) => {
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
                            .send_request(RequestArguments::stackTrace(StackTraceArguments {
                                thread_id: state.current_thread,
                                start_frame: None,
                                levels: None,
                                format: None,
                            }))
                            .await?;
                    }
                }
                ResponseBody::variables(res) => {
                    if let Some(RequestArguments::variables(req)) = req {
                        state
                            .variables
                            .insert(req.variables_reference, res.variables);
                    }
                }
                ResponseBody::Async => todo!(),
                ResponseBody::cancel => todo!(),
                ResponseBody::attach => todo!(),
                ResponseBody::setBreakpoints(body) => {
                    // actions.push(Action::
                }
                ResponseBody::setFunctionBreakpoints(_) => todo!(),
                ResponseBody::setExceptionBreakpoints => todo!(),
                ResponseBody::pause => todo!(),
                ResponseBody::next => todo!(),
                ResponseBody::stepOut => todo!(),
                ResponseBody::stepBack => todo!(),
                ResponseBody::reverseContinue => todo!(),
                ResponseBody::source(_) => todo!(),
                ResponseBody::completions(_) => todo!(),
                ResponseBody::gotoTargets(_) => todo!(),
                ResponseBody::goto => todo!(),
                ResponseBody::restartFrame => todo!(),
                ResponseBody::evaluate(_) => todo!(),
                ResponseBody::setVariable(_) => todo!(),
                ResponseBody::dataBreakpointInfo(_) => todo!(),
                ResponseBody::setDataBreakpoints(_) => todo!(),
                ResponseBody::readMemory(_) => todo!(),
                ResponseBody::writeMemory(_) => todo!(),
                ResponseBody::terminate => todo!(),
                ResponseBody::disconnect => todo!(),
            };
        }
        ResponseResult::Error {
            command,
            message,
            show_user,
        } => {
            error!("'{command}' response failure: {message}");
        }
    }

    if adapter.num_requests() == 0 {
        actions.push(Action::Redraw);
    }

    Ok(actions)
}
