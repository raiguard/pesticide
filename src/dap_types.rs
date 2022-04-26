// TODO: Don't use anyhow here, this is a library!
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdapterMessage {
    Event(EventPayload),
    // Request(RequestPayload),
    // Response(ResponsePayload),
    Unknown(Value),
}

impl AdapterMessage {
    pub fn from(input: &str) -> Result<Self> {
        let json: Value = serde_json::from_str(input)?;
        let msg_type = json["type"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid DAP message format"))?;

        Ok(match msg_type {
            "event" => {
                let event_name = json["event"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing event identifier"))?;

                let (event_name, body) = match event_name {
                    "output" => ("output".to_string(), {
                        OutputEventBody::deserialize(&json["body"])
                            .ok()
                            .map(EventBody::Output)
                    }),
                    _ => (
                        event_name.to_string(),
                        match json["body"] {
                            Value::Object(_) => Some(EventBody::Unknown(json["body"].clone())),
                            _ => None,
                        },
                    ),
                };

                AdapterMessage::Event(EventPayload {
                    body,
                    event: event_name,
                    seq: json["seq"].as_u64().map(|value| value as u32).unwrap(),
                })
            }
            _ => AdapterMessage::Unknown(json),
        })
    }
}

// EVENTS

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPayload {
    pub body: Option<EventBody>,
    pub event: String,
    pub seq: u32,
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EventBody {
    Output(OutputEventBody),
    Unknown(Value),
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

// Initialize

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
        assert_eq!(
            AdapterMessage::from(json_str).unwrap(),
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
                seq: 1
            })
        );
    }
}
