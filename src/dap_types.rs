use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

/// Base construct for sending messages to/from the debug adapter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum AdapterMessage {
    Event(Event),
    Request(Request),
    Response(Response),
}

/// Some events have empty bodies.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Empty {}

// EVENTS

/// A debug adapter initiated event.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "event")]
#[serde(rename_all = "lowercase")]
pub enum Event {
    Exited(EventPayload<ExitedEvent>),
    Initialized(EventPayload<Empty>),
    Output(EventPayload<OutputEvent>),
    Process(EventPayload<ProcessEvent>),
    Stopped(EventPayload<StoppedEvent>),
    Thread(EventPayload<ThreadEvent>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventPayload<T> {
    /// Sequence number (also known as message ID). For protocol messages of
    /// of type 'Request', this ID can be used to cancel the request.
    pub seq: u32,

    /// Event-specific information.
    pub body: Option<T>,
}

// Exited

/// The event indicates that the debuggee has exited and returns its exit code.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitedEvent {
    /// The exit code returned from the debugee.
    pub exit_code: u32,
}

// Output

/// The event indicates that the target has produced some output.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEvent {
    /// The output category. If not specified or if the category is not
    /// understood by the client, 'console' is assumed.
    pub category: Option<OutputEventCategory>,

    /// The output to report.
    pub output: String,

    /// Support for keeping an output log organized by grouping related
    /// messages.
    pub group: Option<OutputEventGroup>,

    /// If an attribute 'variablesReference' exists and its value is > 0, the
    /// output contains objects which can be retrieved by passing
    /// 'variablesReference' to the 'variables' request. The value should be
    /// less than or equal to 2147483647 (2^31-1).
    pub variables_reference: Option<u32>,

    /// An optional source location where the output was produced.
    pub source: Option<Source>,

    /// An optional source location line where the output was produced.
    pub line: Option<u32>,

    /// An optional source location column where the output was produced.
    pub column: Option<u32>,

    /// Optional data to report. For the 'telemetry' category the data will be
    /// sent to telemetry, for the other categories the data is shown in JSON
    /// format.
    pub data: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputEventCategory {
    /// Show the output in the client's default message UI, e.g. a
    /// 'debug console'. This category should only be used for informational
    /// output from the debugger (as opposed to the debuggee).
    Console,

    /// A hint for the client to show the output in the client's UI
    /// for important and highly visible information, e.g. as a popup
    /// notification. This category should only be used for important messages
    /// from the debugger (as opposed to the debuggee). Since this category
    /// value is a hint, clients might ignore the hint and assume the 'console'
    /// category.
    Important,

    /// Show the output as normal program output from the debuggee.
    Stderr,

    /// Show the output as error program output from the debuggee.
    Stdout,

    /// Send the output to telemetry instead of showing it to the
    /// user.
    Telemetry,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputEventGroup {
    /// Start a new group in expanded mode. Subsequent output events are
    /// members of the group and should be shown indented.
    /// The 'output' attribute becomes the name of the group and is not
    /// indented.
    Start,

    /// Start a new group in collapsed mode. Subsequent output events are
    /// members of the group and should be shown indented (as soon as the
    /// group is expanded).
    /// The 'output' attribute becomes the name of the group and is not
    /// indented.
    StartCollapsed,

    /// 'End the current group and decreases the indentation of subsequent
    /// output events.
    /// A non empty 'output' attribute is shown as the unindented end of the
    /// group.
    End,
}

// Process

/// The event indicates that the debugger has begun debugging a new process.
/// Either one that it has launched, or one that it has attached to.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEvent {
    /// The logical name of the process. This is usually the full path to
    /// process's executable file. _example: /home/example/myproj/program.js.
    pub name: String,

    /// The system process id of the debugged process. This property will be
    /// missing for non-system processes.
    pub system_process_id: Option<u32>,

    /// If true, the process is running on the same computer as the debug
    /// adapter.
    #[serde(default)]
    pub is_local_process: bool,

    /// Describes how the debug engine started debugging this process.
    pub start_method: Option<ProcessStartMethod>,

    /// The size of a pointer or address for this process, in bits. This value
    /// may be used by clients when formatting addresses for display.
    pub pointer_size: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessStartMethod {
    /// Debugger attached to an existing process.
    Attach,

    /// A project launcher component has launched a new process in a suspended
    /// state and then asked the debugger to attach.
    AttachForSuspendedLaunch,

    /// Process was launched under the debugger.
    Launch,
}

// Stopped

/// The event indicates that the execution of the debuggee has stopped due to
/// some condition.
///
/// This can be caused by a break point previously set, a stepping request has
/// completed, by executing a debugger statement etc.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoppedEvent {
    /// The reason for the event.
    /// For backward compatibility this string is shown in the UI if the
    /// 'description' attribute is missing (but it must not be translated).
    pub reason: StoppedReason,

    /// The full reason for the event, e.g. 'Paused on exception'. This string
    /// is shown in the UI as is and must be translated.
    pub description: Option<String>,

    /// The thread which was stopped.
    pub thread_id: u32,

    /// A value of true hints to the frontend that this event should not change
    /// the focus.
    #[serde(default)]
    pub preserve_focus_hint: bool,

    /// Additional information. E.g. if reason is 'exception', text contains
    /// the exception name. This string is shown in the UI.
    pub text: Option<String>,

    /// If 'allThreadsStopped' is true, a debug adapter can announce that all
    /// threads have stopped.
    /// - The client should use this information to enable that all threads can
    /// be expanded to access their stacktraces.
    /// - If the attribute is missing or false, only the thread with the given
    /// threadId can be expanded.
    #[serde(default)]
    pub all_threads_stopped: bool,

    /// Ids of the breakpoints that triggered the event. In most cases there
    /// will be only a single breakpoint but here are some examples for
    /// multiple breakpoints:
    /// - Different types of breakpoints map to the same location.
    /// - Multiple source breakpoints get collapsed to the same instruction by
    /// the compiler/runtime.
    /// - Multiple function breakpoints with different function names map to
    /// the same location.
    pub hit_breakpoint_ids: Option<Vec<u32>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StoppedReason {
    Step,
    Breakpoint,
    Exception,
    Pause,
    Entry,
    Goto,
    FunctionBreakpoint,
    DataBreakpoint,
    InstructionBreakpoint,
}

// Thread

/// The event indicates that a thread has started or exited.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadEvent {
    /// The reason for the event.
    pub reason: ThreadReason,

    /// The identifier of the thread.
    pub thread_id: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThreadReason {
    Started,
    Exited,
}

// REQUESTS

/// A client or debug adapter initiated request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "command")]
#[serde(rename_all = "camelCase")]
pub enum Request {
    ConfigurationDone(RequestPayload<Empty>),
    Initialize(RequestPayload<InitializeRequest>),
    Launch(RequestPayload<Value>),
    RunInTerminal(RequestPayload<RunInTerminalRequest>),
    SetBreakpoints(RequestPayload<SetBreakpointsRequest>),
    StepIn(RequestPayload<StepInRequest>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RequestPayload<T> {
    /// Sequence number (also known as message ID). For protocol messages of
    /// of type 'Request', this ID can be used to cancel the request.
    pub seq: u32,

    /// Object containing arguments for the command.
    #[serde(rename = "arguments")]
    pub args: Option<T>,
}

// Initialize

/// The 'initialize' request is sent as the first request from the client to
/// the debug adapter in order to configure it with client capabilities and to
/// retrieve capabilities from the debug adapter.
///
/// Until the debug adapter has responded to with an 'initialize' response, the
/// client must not send any additional requests or events to the debug
/// adapter.
///
/// In addition the debug adapter is not allowed to send any requests or events
/// to the client until it has responded with an 'initialize' response.
///
/// The 'initialize' request may only be sent once.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequest {
    /// The ID of the (frontend) client using this adapter.
    #[serde(rename = "clientID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,

    /// The human readable name of the (frontend) client using this adapter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,

    /// The ID of the debug adapter.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "adapterID")]
    pub adapter_id: Option<String>,

    /// The ISO-639 locale of the (frontend) client using this adapter, e.g.
    /// en-US or de-CH.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,

    /// If true all line numbers are 1-based (default).
    #[serde(default = "default_as_true")]
    pub lines_start_at_1: bool,

    /// If true all column numbers are 1-based (default).
    #[serde(default = "default_as_true")]
    pub columns_start_at_1: bool,

    /// Determines in what format paths are specified. The default is 'path',
    /// which is the native format.
    pub path_format: Option<InitializeRequestPathFormat>,

    /// Client supports the optional type attribute for variables.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub supports_variable_type: bool,

    /// Client supports the paging of variables.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub supports_variable_paging: bool,

    /// Client supports the runInTerminal request.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub supports_run_in_terminal_request: bool,

    /// Client supports memory references.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub supports_memory_references: bool,

    /// Client supports progress reporting.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub supports_progress_reporting: bool,

    /// Client supports the invalidated event.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub supports_invalidated_event: bool,

    /// Client supports the memory event.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub supports_memory_event: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InitializeRequestPathFormat {
    Path,
    Uri,
}

// RunInTerminal

/// This optional request is sent from the debug adapter to the client to run
/// a command in a terminal.
///
/// This is typically used to launch the debuggee in a terminal provided by the
/// client.
///
/// This request should only be called if the client has passed the value true
/// for the ‘supportsRunInTerminalRequest’ capability of the ‘initialize’
/// request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunInTerminalRequest {
    /// What kind of terminal to launch.
    pub kind: RunInTerminalKind,

    /// Optional title of the terminal.
    pub title: Option<String>,

    /// Working directory for the command. For non-empty, valid paths this
    /// typically results in execution of a change directory command.
    pub cwd: String,

    /// List of arguments. The first argument is the command to run.
    pub args: Vec<String>,

    /// Environment key-value pairs that are added to or removed from the
    /// default environment.
    pub env: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RunInTerminalKind {
    External,
    Integrated,
}

// SetBreakpoints

/// Sets multiple breakpoints for a single source and clears all previous
/// breakpoints in that source.
///
/// To clear all breakpoint for a source, specify an empty array.
///
/// When a breakpoint is hit, a ‘stopped’ event (with reason ‘breakpoint’) is
/// generated.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointsRequest {
    // The source location of the breakpoints; either 'source.path' or
    // 'source.reference' must be specified.
    source: Source,

    // The code locations of the breakpoints.
    breakpoints: Vec<SourceBreakpoint>,

    // Deprecated: The code locations of the breakpoints.
    lines: Option<Vec<u32>>,

    // A value of true indicates that the underlying source has been modified
    // which results in new breakpoint locations.
    #[serde(default)]
    source_modified: bool,
}

/// The request resumes the given thread to step into a function/method and
/// allows all other threads to run freely by resuming them.
///
/// If the debug adapter supports single thread execution (see capability
/// 'supportsSingleThreadExecutionRequests') setting the 'singleThread'
/// argument to true prevents other suspended threads from resuming.
///
/// If the request cannot step into a target, 'stepIn' behaves like the 'next'
/// request.
///
/// The debug adapter first sends the response and then a 'stopped' event (with
///  reason 'step') after the step has completed.
///
/// If there are multiple function/method calls (or other targets) on the
/// source line,
///
/// the optional argument 'targetId' can be used to control into which target
/// the 'stepIn' should occur.
///
/// The list of possible targets for a given source line can be retrieved via
/// the 'stepInTargets' request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepInRequest {
    /// Specifies the thread for which to resume execution for one step-into
    /// (of the given granularity).
    pub thread_id: u32,

    /// If this optional flag is true, all other suspended threads are not
    /// resumed.
    #[serde(default)]
    pub single_thread: bool,

    /// Optional id of the target to step into.
    pub target_id: Option<u32>,

    /// Optional granularity to step. If no granularity is specified, a
    /// granularity of 'Statement' is assumed.
    #[serde(default = "stepping_granularity_default")]
    pub granularity: SteppingGranularity,
}

// RESPONSES

/// Response for a request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "command")]
#[serde(rename_all = "camelCase")]
pub enum Response {
    ConfigurationDone(ResponsePayload<Empty>),
    Initialize(ResponsePayload<Capabilities>),
    Launch(ResponsePayload<Empty>),
    RunInTerminal(ResponsePayload<RunInTerminalResponse>),
    StepIn(ResponsePayload<Empty>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponsePayload<T> {
    /// Sequence number (also known as message ID). For protocol messages of
    /// of type 'Request', this ID can be used to cancel the request.
    pub seq: u32,

    /// Sequence number of the corresponding request.
    pub request_seq: u32,

    /// Outcome of the request.
    ///
    /// If true, the request was successful and the 'body' attribute may
    /// contain the result of the request.
    ///
    /// If the value is false, the attribute 'message' contains the error in
    /// short form and the 'body' may contain additional information (see
    /// 'ErrorResponse.body.error').
    pub success: bool,

    /// Contains the raw error in short form if 'success' is false.
    /// This raw error might be interpreted by the frontend and is not shown
    /// in the UI.
    pub message: Option<String>,

    /// Contains request result if success is true and optional error details
    /// if success is false.
    pub body: Option<T>,
}

// RunInTerminal

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RunInTerminalResponse {
    /// The process ID. The value should be less than or equal to 2147483647
    /// (2^31-1).
    #[serde(rename = "processID")]
    pub process_id: Option<u32>,

    /// The process ID of the terminal shell. The value should be less than or
    /// equal to 2147483647 (2^31-1).
    #[serde(rename = "shellProcessID")]
    pub shell_process_id: Option<u32>,
}

// TYPES

/// Information about the capabilities of a debug adapter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    /// The debug adapter supports the 'configurationDone' request.
    #[serde(default)]
    supports_configuration_done_request: bool,

    /// The debug adapter supports function breakpoints.
    #[serde(default)]
    supports_function_breakpoints: bool,

    /// The debug adapter supports conditional breakpoints.
    #[serde(default)]
    supports_conditional_breakpoints: bool,

    /// The debug adapter supports breakpoints that break execution after a
    /// specified number of hits.
    #[serde(default)]
    supports_hit_conditional_breakpoints: bool,

    /// The debug adapter supports a (side effect free) evaluate request for
    /// data hovers.
    #[serde(default)]
    supports_evaluate_for_hovers: bool,

    /// Available exception filter options for the 'setExceptionBreakpoints'
    /// request.
    #[serde(default)]
    exception_breakpoint_filters: Option<Vec<ExceptionBreakpointsFilter>>,

    /// The debug adapter supports stepping back via the 'stepBack' and
    /// 'reverseContinue' requests.
    #[serde(default)]
    supports_step_back: bool,

    /// The debug adapter supports setting a variable to a value.
    #[serde(default)]
    supports_set_variable: bool,

    /// The debug adapter supports restarting a frame.
    #[serde(default)]
    supports_restart_frame: bool,

    /// The debug adapter supports the 'gotoTargets' request.
    #[serde(default)]
    supports_goto_targets_request: bool,

    /// The debug adapter supports the 'stepInTargets' request.
    #[serde(default)]
    supports_step_in_targets_request: bool,

    /// The debug adapter supports the 'completions' request.
    #[serde(default)]
    supports_completions_request: bool,

    /// The set of characters that should trigger completion in a REPL. If not
    /// specified, the UI should assume the '.' character.
    #[serde(default)]
    completion_trigger_characters: bool,

    /// The debug adapter supports the 'modules' request.
    #[serde(default)]
    supports_modules_request: bool,

    /// The set of additional module information exposed by the debug adapter.
    #[serde(default)]
    additional_module_columns: bool,

    /// Checksum algorithms supported by the debug adapter.
    #[serde(default)]
    supported_checksum_algorithms: bool,

    /// The debug adapter supports the 'restart' request. In this case a client
    /// should not implement 'restart' by terminating and relaunching the
    /// adapter but by calling the RestartRequest.
    #[serde(default)]
    supports_restart_request: bool,

    /// The debug adapter supports 'exceptionOptions' on the
    /// setExceptionBreakpoints request.
    #[serde(default)]
    supports_exception_options: bool,

    /// The debug adapter supports a 'format' attribute on the
    /// stackTraceRequest, variablesRequest, and evaluateRequest.
    #[serde(default)]
    supports_value_formatting_options: bool,

    /// The debug adapter supports the 'exceptionInfo' request.
    #[serde(default)]
    supports_exception_info_request: bool,

    /// The debug adapter supports the 'terminateDebuggee' attribute on the
    /// 'disconnect' request.
    #[serde(default)]
    support_terminate_debuggee: bool,

    /// The debug adapter supports the 'suspendDebuggee' attribute on the
    /// 'disconnect' request.
    #[serde(default)]
    support_suspend_debuggee: bool,

    /// The debug adapter supports the delayed loading of parts of the stack,
    /// which requires that both the 'startFrame' and 'levels' arguments and an
    /// optional 'totalFrames' result of the 'StackTrace' request are
    /// supported.
    #[serde(default)]
    supports_delayed_stack_trace_loading: bool,

    /// The debug adapter supports the 'loadedSources' request.
    #[serde(default)]
    supports_loaded_sources_request: bool,

    /// The debug adapter supports logpoints by interpreting the 'logMessage'
    /// attribute of the SourceBreakpoint.
    #[serde(default)]
    supports_log_points: bool,

    /// The debug adapter supports the 'terminateThreads' request.
    #[serde(default)]
    supports_terminate_threads_request: bool,

    /// The debug adapter supports the 'setExpression' request.
    #[serde(default)]
    supports_set_expression: bool,

    /// The debug adapter supports the 'terminate' request.
    #[serde(default)]
    supports_terminate_request: bool,

    /// The debug adapter supports data breakpoints.
    #[serde(default)]
    supports_data_breakpoints: bool,

    /// The debug adapter supports the 'readMemory' request.
    #[serde(default)]
    supports_read_memory_request: bool,

    /// The debug adapter supports the 'writeMemory' request.
    #[serde(default)]
    supports_write_memory_request: bool,

    /// The debug adapter supports the 'disassemble' request.
    #[serde(default)]
    supports_disassemble_request: bool,

    /// The debug adapter supports the 'cancel' request.
    #[serde(default)]
    supports_cancel_request: bool,

    /// The debug adapter supports the 'breakpointLocations' request.
    #[serde(default)]
    supports_breakpoint_locations_request: bool,

    /// The debug adapter supports the 'clipboard' context value in the
    /// 'evaluate' request.
    #[serde(default)]
    supports_clipboard_context: bool,

    /// The debug adapter supports stepping granularities (argument
    /// 'granularity') for the stepping requests.
    #[serde(default)]
    supports_stepping_granularity: bool,

    /// The debug adapter supports adding breakpoints based on instruction
    /// references.
    #[serde(default)]
    supports_instruction_breakpoints: bool,

    /// The debug adapter supports 'filterOptions' as an argument on the
    /// 'setExceptionBreakpoints' request.
    #[serde(default)]
    supports_exception_filter_options: bool,

    /// The debug adapter supports the 'singleThread' property on the execution
    /// requests ('continue', 'next', 'stepIn', 'stepOut', 'reverseContinue',
    /// 'stepBack').
    #[serde(default)]
    supports_single_thread_execution_requests: bool,
}

/// The checksum of an item calculated by the specified algorithm.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checksum {
    /// The algorithm used to calculate this checksum.
    pub algorithm: ChecksumAlgorithm,

    /// Value of the checksum.
    pub checksum: String,
}

/// Names of checksum algorithms that may be supported by a debug adapter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ChecksumAlgorithm {
    MD5,
    SHA1,
    SHA256,
    #[serde(rename = "lowercase")]
    Timestamp,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SourcePresentationHint {
    Normal,
    Emphasize,
    Deemphasize,
}

/// An ExceptionBreakpointsFilter is shown in the UI as an filter option for
/// configuring how exceptions are dealt with.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionBreakpointsFilter {
    /// The internal ID of the filter option. This value is passed to the
    /// 'setExceptionBreakpoints' request.
    filter: String,

    /// The name of the filter option. This will be shown in the UI.
    label: String,

    /// An optional help text providing additional information about the
    /// exception filter. This String is typically shown as a hover and must be
    /// translated.
    description: Option<String>,

    /// Initial value of the filter option. If not specified a value 'false' is
    /// assumed.
    #[serde(default)]
    default: bool,

    /// Controls whether a condition can be specified for this filter option.
    /// If false or missing, a condition can not be set.
    #[serde(default)]
    supports_condition: bool,

    /// An optional help text providing information about the condition. This
    /// string is shown as the placeholder text for a text box and must be
    /// translated.
    condition_description: Option<String>,
}

/// A Source is a descriptor for source code.
///
/// It is returned from the debug adapter as part of a StackFrame and it is
/// used by clients when specifying breakpoints.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    /// The short name of the source. Every source returned from the debug
    /// adapter has a name.
    ///
    /// When sending a source to the debug adapter this name is optional.
    pub name: String,

    /// The path of the source to be shown in the UI.
    /// It is only used to locate and load the content of the source if no
    /// sourceReference is specified (or its value is 0).
    pub path: Option<PathBuf>,

    /// If sourceReference > 0 the contents of the source must be retrieved
    /// through the SourceRequest (even if a path is specified).
    ///
    /// A sourceReference is only valid for a session, so it must not be used
    /// to persist a source.
    ///
    /// The value should be less than or equal to 2147483647 (2^31-1).
    pub source_reference: Option<u32>,

    /// An optional hint for how to present the source in the UI.
    /// A value of 'deemphasize' can be used to indicate that the source is not
    /// available or that it is skipped on stepping.
    pub presentation_hint: Option<SourcePresentationHint>,

    /// The (optional) origin of this source: possible values 'internal
    /// module', 'inlined content from source map', etc.
    pub origin: Option<String>,

    /// An optional list of sources that are related to this source. These may
    /// be the source that generated this source.
    pub sources: Option<Vec<Source>>,

    /// Optional data that a debug adapter might want to loop through the
    /// client.
    ///
    /// The client should leave the data intact and persist it across sessions.
    /// The client should not interpret the data.
    pub adapter_data: Option<Value>,

    /// The checksums associated with this file.
    pub checksums: Option<Vec<Checksum>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakpoint {
    /// The source line of the breakpoint or logpoint.
    pub line: Option<u32>,

    /// An optional source column of the breakpoint.
    pub column: Option<u32>,

    /// An optional expression for conditional breakpoints.
    /// It is only honored by a debug adapter if the capability
    /// 'supportsConditionalBreakpoints' is true.
    pub condition: Option<String>,

    /// An optional expression that controls how many hits of the breakpoint
    /// are ignored.
    /// The backend is expected to interpret the expression as needed.
    /// The attribute is only honored by a debug adapter if the capability
    /// 'supportsHitConditionalBreakpoints' is true.
    pub hit_condition: Option<String>,

    /// If this attribute exists and is non-empty, the backend must not 'break'
    /// (stop)
    /// but log the message instead. Expressions within {} are interpolated.
    /// The attribute is only honored by a debug adapter if the capability
    /// 'supportsLogPoints' is true.
    pub log_message: Option<String>,
}

/// The granularity of one 'step' in the stepping requests 'next', 'stepIn',
/// 'stepOut', and 'stepBack'.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SteppingGranularity {
    /// The step should allow the program to run until the current statement
    /// has finished executing. The meaning of a statement is determined by
    /// the adapter and it may be considered equivalent to a line. For example,
    /// `for(int i = 0; i < 10; i++)` could be considered to have 3 statements:
    /// `int i = 0`, `i < 10`, and `i++`.
    Statement,
    /// The step should allow the program to run until the current source line
    /// has executed.
    Line,
    /// The step should allow one instruction to execute (e.g. one x86
    /// instruction). etc.
    Instruction,
}

// This is ugly and sad
fn stepping_granularity_default() -> SteppingGranularity {
    SteppingGranularity::Statement
}

// UTILITIES

fn default_as_true() -> bool {
    true
}

fn is_false(boolean: &bool) -> bool {
    !boolean
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_event() {
        let json_str = r#"
            {
                "type": "event",
                "event": "output",
                "seq": 1,
                "body": {
                    "category": "console",
                    "output": "Hello world!"
                }
            }
        "#;
        let msg: AdapterMessage = serde_json::from_str(json_str).unwrap();
        println!("{}", serde_json::to_string_pretty(&msg).unwrap());
        assert_eq!(
            msg,
            AdapterMessage::Event(Event::Output(EventPayload {
                body: Some(OutputEvent {
                    category: Some(OutputEventCategory::Console),
                    output: "Hello world!".to_string(),
                    group: None,
                    variables_reference: None,
                    source: None,
                    line: None,
                    column: None,
                    data: None
                }),
                seq: 1
            })),
        );
    }

    #[test]
    fn initialize_request() {
        let json_str = r#"
            {
              "type": "request",
              "command": "initialize",
              "seq": 0,
              "arguments": {
                "clientID": "pesticide",
                "clientName": "Pesticide",
                "adapterID": "pydbg",
                "locale": "en-US",
                "linesStartAt1": true,
                "columnsStartAt1": true,
                "pathFormat": "path"
              }
            }
        "#;
        let msg: AdapterMessage = serde_json::from_str(json_str).unwrap();
        println!("{}", serde_json::to_string_pretty(&msg).unwrap());
        assert_eq!(
            msg,
            AdapterMessage::Request(Request::Initialize(RequestPayload {
                seq: 0,
                args: Some(InitializeRequest {
                    client_id: Some("pesticide".to_string()),
                    client_name: Some("Pesticide".to_string()),
                    adapter_id: Some("pydbg".to_string()),
                    locale: Some("en-US".to_string()),
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
            }))
        );
    }

    #[test]
    fn initialize_response() {
        // NOTE: supportsDebuggerProperties and supportsTerminateDebuggee are
        // not official parts of the DAP
        let json_str = r#"
            {
              "seq": 3,
              "type": "response",
              "request_seq": 0,
              "success": true,
              "command": "initialize",
              "body": {
                "supportsCompletionsRequest": true,
                "supportsConditionalBreakpoints": true,
                "supportsConfigurationDoneRequest": true,
                "supportsDebuggerProperties": true,
                "supportsDelayedStackTraceLoading": true,
                "supportsEvaluateForHovers": true,
                "supportsExceptionInfoRequest": true,
                "supportsExceptionOptions": true,
                "supportsFunctionBreakpoints": true,
                "supportsHitConditionalBreakpoints": true,
                "supportsLogPoints": true,
                "supportsModulesRequest": true,
                "supportsSetExpression": true,
                "supportsSetVariable": true,
                "supportsValueFormattingOptions": true,
                "supportsTerminateDebuggee": true,
                "supportsGotoTargetsRequest": true,
                "supportsClipboardContext": true,
                "exceptionBreakpointFilters": [
                  {
                    "filter": "raised",
                    "label": "Raised Exceptions",
                    "default": false,
                    "description": "Break whenever any exception is raised."
                  },
                  {
                    "filter": "uncaught",
                    "label": "Uncaught Exceptions",
                    "default": true,
                    "description": "Break when the process is exiting due to unhandled exception."
                  },
                  {
                    "filter": "userUnhandled",
                    "label": "User Uncaught Exceptions",
                    "default": false,
                    "description": "Break when exception escapes into library code."
                  }
                ],
                "supportsStepInTargetsRequest": true
              }
            }
        "#;
        let msg: AdapterMessage = serde_json::from_str(json_str).unwrap();
        println!("{}", serde_json::to_string_pretty(&msg).unwrap());

        assert_eq!(
            msg,
            AdapterMessage::Response(Response::Initialize(ResponsePayload {
                seq: 3,
                request_seq: 0,
                success: true,
                message: None,
                body: Some(Capabilities {
                    supports_completions_request: true,
                    supports_conditional_breakpoints: true,
                    supports_configuration_done_request: true,
                    supports_delayed_stack_trace_loading: true,
                    supports_evaluate_for_hovers: true,
                    supports_exception_info_request: true,
                    supports_exception_options: true,
                    supports_function_breakpoints: true,
                    supports_hit_conditional_breakpoints: true,
                    supports_log_points: true,
                    supports_modules_request: true,
                    supports_set_expression: true,
                    supports_set_variable: true,
                    supports_value_formatting_options: true,
                    supports_goto_targets_request: true,
                    supports_clipboard_context: true,
                    supports_step_back: false,
                    supports_restart_frame: false,
                    supports_step_in_targets_request: true,
                    completion_trigger_characters: false,
                    additional_module_columns: false,
                    supported_checksum_algorithms: false,
                    supports_restart_request: false,
                    support_terminate_debuggee: false,
                    support_suspend_debuggee: false,
                    supports_loaded_sources_request: false,
                    supports_terminate_threads_request: false,
                    supports_terminate_request: false,
                    supports_data_breakpoints: false,
                    supports_read_memory_request: false,
                    supports_write_memory_request: false,
                    supports_disassemble_request: false,
                    supports_cancel_request: false,
                    supports_breakpoint_locations_request: false,
                    supports_stepping_granularity: false,
                    supports_instruction_breakpoints: false,
                    supports_exception_filter_options: false,
                    supports_single_thread_execution_requests: false,
                    exception_breakpoint_filters: Some(vec![
                        ExceptionBreakpointsFilter {
                            filter: "raised".to_string(),
                            label: "Raised Exceptions".to_string(),
                            description: Some(
                                "Break whenever any exception is raised.".to_string()
                            ),
                            default: false,
                            supports_condition: false,
                            condition_description: None,
                        },
                        ExceptionBreakpointsFilter {
                            filter: "uncaught".to_string(),
                            label: "Uncaught Exceptions".to_string(),
                            description: Some(
                                "Break when the process is exiting due to unhandled exception."
                                    .to_string(),
                            ),
                            default: true,
                            supports_condition: false,
                            condition_description: None,
                        },
                        ExceptionBreakpointsFilter {
                            filter: "userUnhandled".to_string(),
                            label: "User Uncaught Exceptions".to_string(),
                            description: Some(
                                "Break when exception escapes into library code.".to_string(),
                            ),
                            default: false,
                            supports_condition: false,
                            condition_description: None,
                        },
                    ]),
                }),
            }))
        );
    }
}
