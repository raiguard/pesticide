use crate::Cli;
use anyhow::{Context, Result};
use regex::{Captures, Regex};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub adapter: String,
    pub adapter_args: Vec<String>,
    pub adapter_id: Option<String>,
    pub term_cmd: Option<String>,
    // This is different for every debug adapter and so cannot be strictly typed
    pub launch_args: serde_json::Value,
}

impl Config {
    pub fn new(cli: Cli) -> Result<Self> {
        // Resolve path
        let path = if let Some(config) = cli.config {
            config
        } else {
            std::env::current_dir()?.join("pesticide.toml")
        };

        // Get contents and expand environment variables
        let mut contents =
            std::fs::read_to_string(path).context("Failed to read configuration file")?;
        let re = Regex::new(r"\$\{(.*?)\}")?;
        contents = re
            .replace_all(&contents, |caps: &Captures| {
                std::env::var(&caps[1]).unwrap_or_default()
            })
            .to_string();
        contents = contents.replace("$$", "$");

        // Create config object
        let mut config: Config =
            toml::from_str(&contents).context("Failed to parse configuration file")?;
        debug!("{:?}", config);

        if cli.term_cmd.is_some() {
            config.term_cmd = cli.term_cmd;
        }

        Ok(config)
    }
}
