use crate::config::Config;
use anyhow::{Context, Result};
use std::{
    io::{BufReader, BufWriter, Read},
    process::{Child, Command, Stdio},
    thread,
};

pub struct Adapter {
    child: Child,
}

impl Adapter {
    pub fn new(config: Config) -> Result<Self> {
        // Start debug adapter process
        let mut child = Command::new(config.adapter)
            .args(config.adapter_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start debug adapter")?;

        // Log adapter errors
        let mut stderr = BufReader::new(child.stderr.take().context("Failed to open stderr")?);
        thread::spawn(move || loop {
            let mut buf = String::new();
            stderr
                .read_to_string(&mut buf)
                .context("Failed to read stderr")
                .unwrap();
            if buf.is_empty() {
                continue;
            }
            error!("Debug adapter: {}", buf);
        });

        let mut writer = BufWriter::new(child.stdin.take().context("Failed to open stdin")?);
        let mut reader = BufReader::new(child.stdout.take().context("Failed to open stdout")?);

        Ok(Self { child })
    }
}
