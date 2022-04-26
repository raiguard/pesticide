use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum AdapterMessage {
    Event(Event),
    Request(RequestPayload),
    Response(ResponsePayload),
}

// EVENTS

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "event")]
#[serde(rename_all = "lowercase")]
pub enum Event {
    Output(EventPayload<OutputEvent>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventPayload<T> {
    pub seq: u32,
    pub body: T,
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

// REQUESTS

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RequestPayload {
    #[serde(rename = "arguments")]
    pub args: Option<RequestArgs>,
    pub command: String,
    pub seq: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum RequestArgs {
    Initialize(InitializeRequestArgs),
}

// Initialize

fn default_as_true() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequestArgs {
    #[serde(rename = "clientID")]
    pub client_id: Option<String>,
    pub client_name: Option<String>,
    pub adapter_id: Option<String>,
    #[serde(default = "default_as_true")]
    pub lines_start_at_1: bool,
    #[serde(default = "default_as_true")]
    pub columns_start_at_1: bool,
    pub path_format: Option<InitializeRequestPathFormat>,
    #[serde(default)]
    pub supports_variable_type: bool,
    #[serde(default)]
    pub supports_variable_paging: bool,
    #[serde(default)]
    pub supports_run_in_terminal_request: bool,
    #[serde(default)]
    pub supports_memory_references: bool,
    #[serde(default)]
    pub supports_progress_reporting: bool,
    #[serde(default)]
    pub supports_invalidated_event: bool,
    #[serde(default)]
    pub supports_memory_event: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InitializeRequestPathFormat {
    Path,
    Uri,
}

// RESPONSES

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponsePayload {
    // TODO: Find a way to get rid of these - they're redundant with the enums
    #[serde(rename = "type")]
    pub id: String,
    pub seq: u32,
    pub request_seq: u32,
    pub success: bool,
    pub command: String,
    pub message: Option<String>,
    pub body: Option<ResponseBody>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ResponseBody {
    Initialized(Capabilities),
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
    exception_breakpoint_filters: bool,

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

// TESTS

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_event() {
        let json_str = r#"{
            "type": "event",
            "event": "output",
            "seq": 1,
            "body": {
                "category": "console",
                "output": "Hello world!"
            }
        }"#;
        let msg: AdapterMessage = serde_json::from_str(json_str).unwrap();
        println!("{}", serde_json::to_string_pretty(&msg).unwrap());
        assert_eq!(
            msg,
            AdapterMessage::Event(Event::Output(EventPayload {
                body: OutputEvent {
                    category: Some(OutputEventCategory::Console),
                    output: "Hello world!".to_string(),
                    group: None,
                    variables_reference: None,
                    source: None,
                    line: None,
                    column: None,
                    data: None
                },
                seq: 1
            })),
        );
    }
}
