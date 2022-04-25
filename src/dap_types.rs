use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

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

// EVENTS

pub const EVENT: &str = "event";
pub trait Event {
    type Body: DeserializeOwned + Serialize;
    const TYPE: &'static str;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EmptyEventBody();

// Initialized

#[derive(Debug)]
pub struct InitializedEvent {}

impl Event for InitializedEvent {
    type Body = EmptyEventBody;
    const TYPE: &'static str = "initialized";
}

// Output

#[derive(Debug)]
pub struct OutputEvent {}

impl Event for OutputEvent {
    type Body = OutputEventBody;
    const TYPE: &'static str = "output";
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEventBody {
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

pub const REQUEST: &str = "request";
pub trait Request {
    type Args: DeserializeOwned + Serialize;
    const COMMAND: &'static str;
}

// Initialize

#[derive(Debug)]
pub struct InitializeRequest {}

impl Request for InitializeRequest {
    type Args = InitializeRequestArgs;
    const COMMAND: &'static str = "initialize";
}

fn default_as_true() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequestArgs {
    #[serde(rename = "clientID")]
    client_id: Option<String>,
    client_name: Option<String>,
    adapter_id: Option<String>,
    #[serde(default = "default_as_true")]
    lines_start_at_1: bool,
    #[serde(default = "default_as_true")]
    columns_start_at_1: bool,
    path_format: Option<InitializeRequestPathFormat>,
    #[serde(default)]
    supports_variable_type: bool,
    #[serde(default)]
    supports_variable_paging: bool,
    #[serde(default)]
    supports_run_in_terminal_request: bool,
    #[serde(default)]
    supports_memory_references: bool,
    #[serde(default)]
    supports_progress_reporting: bool,
    #[serde(default)]
    supports_invalidated_event: bool,
    #[serde(default)]
    supports_memory_event: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InitializeRequestPathFormat {
    Path,
    Uri,
}
