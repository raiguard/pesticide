use crate::config::Config;
use anyhow::{bail, Context, Result};
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::process::{Child, Command, Stdio};
use std::thread;

pub struct Adapter {
    pub child: Child,
    pub rx: Receiver<Value>,

    next_seq: u32,
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

        let stdout = BufReader::new(child.stdout.take().context("Failed to open stdout")?);
        let (out_tx, out_rx) = crossbeam_channel::bounded(1024);
        thread::spawn(move || {
            reader_loop(stdout, &out_tx).expect("Failed to read message from debug adapter");
        });

        let stdin = BufWriter::new(child.stdin.take().context("Failed to open stdin")?);
        let (in_tx, in_rx) = crossbeam_channel::bounded(1024);
        thread::spawn(move || {
            writer_loop(stdin, &in_rx).expect("Failed to read message from debug adapter");
        });

        Ok(Self {
            child,
            rx: out_rx,
            next_seq: 0,
        })
    }
}

// Thread to read the stdout of the debug adapter process.
fn reader_loop(mut reader: impl BufRead, tx: &Sender<Value>) -> Result<()> {
    let mut headers = HashMap::new();
    loop {
        // Parse headers
        headers.clear();
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header)? == 0 {
                debug!("Debug adapter closed pipe, stopping reading");
                return Ok(());
            }
            let header = header.trim();
            if header.is_empty() {
                break;
            }
            let parts: Vec<&str> = header.split(": ").collect();
            if parts.len() != 2 {
                bail!("Failed to parse header");
            }
            headers.insert(parts[0].to_string(), parts[1].to_string());
        }
        // Get the length of the message we are receiving
        let content_len = headers
            .get("Content-Length")
            .expect("Failed to find Content-Length header")
            .parse()
            .expect("Failed to parse Content-Length header");
        // Now read that many characters to obtain the message
        let mut content = vec![0; content_len];
        reader.read_exact(&mut content)?;
        let content = String::from_utf8(content).expect("Failed to read content as UTF-8 string");
        let msg: Value = serde_json::from_str(&content).unwrap();
        debug!(
            "From debug adapter: {}",
            // TEMPORARY:
            serde_json::to_string_pretty(&msg)?
        );

        if msg.is_object() {
            tx.send(msg)
                .expect("Failed to send message from debug adapter");
        }
    }
}

// Thread to write to the stdin of the debug adapter process
fn writer_loop<'a, T, W>(mut writer: W, rx: &Receiver<T>) -> Result<()>
where
    T: Deserialize<'a> + Serialize,
    W: Write,
{
    for request in rx {
        let request = serde_json::to_string(&request)?;
        write!(
            writer,
            "Content-Length: {}\r\n\r\n{}",
            request.len(),
            request
        )?;
        writer.flush()?;
    }
    Ok(())
}
