use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum AdapterMessage {
    Event(Event),
    Request(Request),
    Response(Response),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Empty {}

// EVENTS

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "event")]
#[serde(rename_all = "lowercase")]
pub enum Event {
    Exited(EventPayload<ExitedEvent>),
    Initialized(EventPayload<Empty>),
    Output(EventPayload<OutputEvent>),
    Process(EventPayload<ProcessEvent>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventPayload<T> {
    pub seq: u32,
    pub body: Option<T>,
}

// Exited

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitedEvent {
    pub exit_code: u32,
}

// Output

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEvent {
    pub category: Option<OutputEventCategory>,
    pub output: String,
    pub group: Option<OutputEventGroup>,
    pub variables_reference: Option<u32>,
    pub source: Option<Source>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub data: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputEventCategory {
    Console,
    Important,
    Stderr,
    Stdout,
    Telemetry,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputEventGroup {
    Start,
    StartCollapsed,
    End,
}

// Process

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEvent {
    // The logical name of the process. This is usually the full path to
    // process's executable file. _example: /home/example/myproj/program.js.
    name: String,

    // The system process id of the debugged process. This property will be
    // missing for non-system processes.
    system_process_id: Option<u32>,

    // If true, the process is running on the same computer as the debug
    // adapter.
    #[serde(default)]
    is_local_process: bool,

    // Describes how the debug engine started debugging this process.
    start_method: Option<ProcessStartMethod>,

    // The size of a pointer or address for this process, in bits. This value
    // may be used by clients when formatting addresses for display.
    pointer_size: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessStartMethod {
    // Debugger attached to an existing process.
    Attach,
    // A project launcher component has launched a new process in a suspended state and then asked the debugger to attach.
    AttachForSuspendedLaunch,
    // Process was launched under the debugger.
    Launch,
}

// REQUESTS

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "command")]
#[serde(rename_all = "camelCase")]
pub enum Request {
    ConfigurationDone(RequestPayload<Empty>),
    Initialize(RequestPayload<InitializeRequest>),
    Launch(RequestPayload<Value>),
    RunInTerminal(RequestPayload<RunInTerminalRequest>),
    SetBreakpoints(RequestPayload<SetBreakpointsRequest>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RequestPayload<T> {
    pub seq: u32,
    #[serde(rename = "arguments")]
    pub args: Option<T>,
}

// Initialize

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequest {
    #[serde(rename = "clientID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "adapterID")]
    pub adapter_id: Option<String>,
    #[serde(default = "default_as_true")]
    pub lines_start_at_1: bool,
    #[serde(default = "default_as_true")]
    pub columns_start_at_1: bool,
    pub path_format: Option<InitializeRequestPathFormat>,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub supports_variable_type: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub supports_variable_paging: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub supports_run_in_terminal_request: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub supports_memory_references: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub supports_progress_reporting: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub supports_invalidated_event: bool,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunInTerminalRequest {
    // What kind of terminal to launch.
    pub kind: RunInTerminalKind,

    // Optional title of the terminal.
    pub title: Option<String>,

    // Working directory for the command. For non-empty, valid paths this
    // typically results in execution of a change directory command.
    pub cwd: String,

    // List of arguments. The first argument is the command to run.
    pub args: Vec<String>,

    // Environment key-value pairs that are added to or removed from the default
    //  environment.
    pub env: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RunInTerminalKind {
    External,
    Integrated,
}

// SetBreakpoints

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

// RESPONSES

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "command")]
#[serde(rename_all = "camelCase")]
pub enum Response {
    ConfigurationDone(ResponsePayload<Empty>),
    Initialize(ResponsePayload<Capabilities>),
    Launch(ResponsePayload<Empty>),
    RunInTerminal(ResponsePayload<RunInTerminalResponse>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponsePayload<T> {
    pub seq: u32,
    pub request_seq: u32,
    pub success: bool,
    // An optional error message if `success` is false
    pub message: Option<String>,
    pub body: Option<T>,
}

// RunInTerminal

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RunInTerminalResponse {
    #[serde(rename = "processID")]
    pub process_id: Option<u32>,
    #[serde(rename = "shellProcessID")]
    pub shell_process_id: Option<u32>,
}

// TYPES

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    // The debug adapter supports the 'configurationDone' request.
    #[serde(default)]
    supports_configuration_done_request: bool,

    // The debug adapter supports function breakpoints.
    #[serde(default)]
    supports_function_breakpoints: bool,

    // The debug adapter supports conditional breakpoints.
    #[serde(default)]
    supports_conditional_breakpoints: bool,

    // The debug adapter supports breakpoints that break execution after a
    // specified number of hits.
    #[serde(default)]
    supports_hit_conditional_breakpoints: bool,

    // The debug adapter supports a (side effect free) evaluate request for data
    // hovers.
    #[serde(default)]
    supports_evaluate_for_hovers: bool,

    // Available exception filter options for the 'setExceptionBreakpoints'
    // request.
    #[serde(default)]
    exception_breakpoint_filters: Option<Vec<ExceptionBreakpointsFilter>>,

    // The debug adapter supports stepping back via the 'stepBack' and
    // 'reverseContinue' requests.
    #[serde(default)]
    supports_step_back: bool,

    // The debug adapter supports setting a variable to a value.
    #[serde(default)]
    supports_set_variable: bool,

    // The debug adapter supports restarting a frame.
    #[serde(default)]
    supports_restart_frame: bool,

    // The debug adapter supports the 'gotoTargets' request.
    #[serde(default)]
    supports_goto_targets_request: bool,

    // The debug adapter supports the 'stepInTargets' request.
    #[serde(default)]
    supports_step_in_targets_request: bool,

    // The debug adapter supports the 'completions' request.
    #[serde(default)]
    supports_completions_request: bool,

    // The set of characters that should trigger completion in a REPL. If not
    // specified, the UI should assume the '.' character.
    #[serde(default)]
    completion_trigger_characters: bool,

    // The debug adapter supports the 'modules' request.
    #[serde(default)]
    supports_modules_request: bool,

    // The set of additional module information exposed by the debug adapter.
    #[serde(default)]
    additional_module_columns: bool,

    // Checksum algorithms supported by the debug adapter.
    #[serde(default)]
    supported_checksum_algorithms: bool,

    // The debug adapter supports the 'restart' request. In this case a client
    // should not implement 'restart' by terminating and relaunching the adapter
    // but by calling the RestartRequest.
    #[serde(default)]
    supports_restart_request: bool,

    // The debug adapter supports 'exceptionOptions' on the
    // setExceptionBreakpoints request.
    #[serde(default)]
    supports_exception_options: bool,

    // The debug adapter supports a 'format' attribute on the stackTraceRequest,
    // variablesRequest, and evaluateRequest.
    #[serde(default)]
    supports_value_formatting_options: bool,

    // The debug adapter supports the 'exceptionInfo' request.
    #[serde(default)]
    supports_exception_info_request: bool,

    // The debug adapter supports the 'terminateDebuggee' attribute on the
    // 'disconnect' request.
    #[serde(default)]
    support_terminate_debuggee: bool,

    // The debug adapter supports the 'suspendDebuggee' attribute on the
    // 'disconnect' request.
    #[serde(default)]
    support_suspend_debuggee: bool,

    // The debug adapter supports the delayed loading of parts of the stack,
    // which requires that both the 'startFrame' and 'levels' arguments and an
    // optional 'totalFrames' result of the 'StackTrace' request are supported.
    #[serde(default)]
    supports_delayed_stack_trace_loading: bool,

    // The debug adapter supports the 'loadedSources' request.
    #[serde(default)]
    supports_loaded_sources_request: bool,

    // The debug adapter supports logpoints by interpreting the 'logMessage'
    // attribute of the SourceBreakpoint.
    #[serde(default)]
    supports_log_points: bool,

    // The debug adapter supports the 'terminateThreads' request.
    #[serde(default)]
    supports_terminate_threads_request: bool,

    // The debug adapter supports the 'setExpression' request.
    #[serde(default)]
    supports_set_expression: bool,

    // The debug adapter supports the 'terminate' request.
    #[serde(default)]
    supports_terminate_request: bool,

    // The debug adapter supports data breakpoints.
    #[serde(default)]
    supports_data_breakpoints: bool,

    // The debug adapter supports the 'readMemory' request.
    #[serde(default)]
    supports_read_memory_request: bool,

    // The debug adapter supports the 'writeMemory' request.
    #[serde(default)]
    supports_write_memory_request: bool,

    // The debug adapter supports the 'disassemble' request.
    #[serde(default)]
    supports_disassemble_request: bool,

    // The debug adapter supports the 'cancel' request.
    #[serde(default)]
    supports_cancel_request: bool,

    // The debug adapter supports the 'breakpointLocations' request.
    #[serde(default)]
    supports_breakpoint_locations_request: bool,

    // The debug adapter supports the 'clipboard' context value in the
    // 'evaluate' request.
    #[serde(default)]
    supports_clipboard_context: bool,

    // The debug adapter supports stepping granularities (argument
    // 'granularity') for the stepping requests.
    #[serde(default)]
    supports_stepping_granularity: bool,

    // The debug adapter supports adding breakpoints based on instruction
    // references.
    #[serde(default)]
    supports_instruction_breakpoints: bool,

    // The debug adapter supports 'filterOptions' as an argument on the
    // 'setExceptionBreakpoints' request.
    #[serde(default)]
    supports_exception_filter_options: bool,

    // The debug adapter supports the 'singleThread' property on the execution
    // requests ('continue', 'next', 'stepIn', 'stepOut', 'reverseContinue',
    // 'stepBack').
    #[serde(default)]
    supports_single_thread_execution_requests: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionBreakpointsFilter {
    // The internal ID of the filter option. This value is passed to the
    // 'setExceptionBreakpoints' request.
    filter: String,

    // The name of the filter option. This will be shown in the UI.
    label: String,

    // An optional help text providing additional information about the exception
    // filter. This String is typically shown as a hover and must be translated.
    description: Option<String>,

    // Initial value of the filter option. If not specified a value 'false' is
    // assumed.
    #[serde(default)]
    default: bool,

    // Controls whether a condition can be specified for this filter option. If
    // false or missing, a condition can not be set.
    #[serde(default)]
    supports_condition: bool,

    // An optional help text providing information about the condition. This
    // string is shown as the placeholder text for a text box and must be
    // translated.
    condition_description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub name: String,
    pub path: Option<PathBuf>,
    pub source_reference: Option<u32>,
    pub presentation_hint: Option<SourcePresentationHint>,
    pub origin: Option<String>,
    pub sources: Option<Vec<Source>>,
    pub adapter_data: Option<Value>,
    pub checksums: Option<Vec<Checksum>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakpoint {
    // The source line of the breakpoint or logpoint.
    pub line: Option<u32>,

    // An optional source column of the breakpoint.
    pub column: Option<u32>,

    // An optional expression for conditional breakpoints.
    // It is only honored by a debug adapter if the capability
    // 'supportsConditionalBreakpoints' is true.
    pub condition: Option<String>,

    // An optional expression that controls how many hits of the breakpoint are
    // ignored.
    // The backend is expected to interpret the expression as needed.
    // The attribute is only honored by a debug adapter if the capability
    // 'supportsHitConditionalBreakpoints' is true.
    pub hit_condition: Option<String>,

    // If this attribute exists and is non-empty, the backend must not 'break'
    // (stop)
    // but log the message instead. Expressions within {} are interpolated.
    // The attribute is only honored by a debug adapter if the capability
    // 'supportsLogPoints' is true.
    pub log_message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checksum {
    pub algorithm: ChecksumAlgorithm,
    pub checksum: String,
}

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
        // NOTE: supportsDebuggerProperties and supportsTerminateDebuggee are not official parts of the DAP
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
