use anyhow::{Context, Result};
use regex::{Captures, Regex};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub adapter: String,
    pub adapter_args: Vec<String>,
    pub adapter_id: Option<String>,
    // This will be passed to the debug adapter as JSON, and will be different for every adapter
    pub launch_args: toml::Value,
}

impl Config {
    pub fn new(path: &Option<PathBuf>) -> Result<Self> {
        // Resolve path
        let path = if let Some(config) = path {
            config.clone()
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
        let config: Config =
            toml::from_str(&contents).context("Failed to parse configuration file")?;
        debug!("{:#?}", config);

        Ok(config)
    }
}
