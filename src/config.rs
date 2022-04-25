use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    adapter: String,
    adapter_args: Vec<String>,
    adapter_id: String,
    // This will be passed to the debug adapter as JSON, and will be different for every adapter
    launch_args: toml::Value,
}
