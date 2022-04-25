use crate::config::Config;
use anyhow::{Context, Result};
use std::{
    io::{BufReader, BufWriter, Read},
    process::{Command, Stdio},
    thread,
};

pub fn start_debugging(config: Config) -> Result<()> {
    // Start debug adapter process
    let mut adapter = Command::new(config.adapter)
        .args(config.adapter_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start debug adapter")?;

    // Create pipes
    let mut writer = BufWriter::new(adapter.stdin.take().context("Failed to open stdin")?);
    let mut reader = BufReader::new(adapter.stdout.take().context("Failed to open stdout")?);
    let mut stderr = BufReader::new(adapter.stderr.take().context("Failed to open stderr")?);

    // Log adapter errors
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

    Ok(())
}
