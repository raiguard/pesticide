use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub adapter: String,
    pub adapter_args: Vec<String>,
    pub adapter_id: String,
    // This will be passed to the debug adapter as JSON, and will be different for every adapter
    pub launch_args: toml::Value,
}
