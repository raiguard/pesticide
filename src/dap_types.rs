use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum AdapterMessage {
    Event(EventPayload),
    Request(RequestPayload),
    // Response(ResponsePayload),
}

// EVENTS

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPayload {
    #[serde(rename = "type")]
    pub id: String,
    pub body: Option<EventBody>,
    pub event: String,
    pub seq: u32,
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum EventBody {
    Output(OutputEventBody),
}

// Output

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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RequestPayload {
    #[serde(rename = "arguments")]
    pub args: Option<RequestArgs>,
    #[serde(rename = "type")]
    pub id: String,
    pub command: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

// TYPES

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
        assert_eq!(
            msg,
            AdapterMessage::Event(EventPayload {
                body: Some(EventBody::Output(OutputEventBody {
                    category: Some(OutputEventCategory::Console),
                    output: "Hello world!".to_string(),
                    group: None,
                    variables_reference: None,
                    source: None,
                    line: None,
                    column: None,
                    data: None
                })),
                event: "output".to_string(),
                id: "event".to_string(),
                seq: 1
            })
        );
        println!("{}", serde_json::to_string_pretty(&msg).unwrap());
    }
}
