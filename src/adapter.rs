use crate::config::Config;
use crate::dap_types::*;
use anyhow::{anyhow, bail, Context, Result};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

pub struct Adapter {
    pub child: Child,
    pub config: Config,

    /// Capabilities defined by the adapter
    /// TODO: Wait to construct adapter object until after this is retrieved?
    pub capabilities: Option<Capabilities>,
    /// Responses from the debug adapter will use the seq as an identifier
    requests: HashMap<u32, Request>,

    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    next_seq: u32,
}

impl Adapter {
    pub fn new(config: Config) -> Result<Self> {
        // Start debug adapter process
        let mut child = Command::new(&config.adapter)
            .args(&config.adapter_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start debug adapter")?;

        // Log adapter errors
        let mut stderr = BufReader::new(child.stderr.take().context("Failed to open stderr")?);
        tokio::spawn(async move {
            loop {
                let mut buf = String::new();
                stderr
                    .read_line(&mut buf)
                    .await
                    .context("Failed to read stderr")
                    .unwrap();
                if buf.is_empty() {
                    continue;
                }
                error!("[DEBUG ADAPTER] >> {}", buf);
            }
        });

        let stdout = BufReader::new(child.stdout.take().context("Failed to open stdout")?);
        let stdin = BufWriter::new(child.stdin.take().context("Failed to open stdin")?);

        Ok(Self {
            child,
            config,

            requests: HashMap::new(),
            capabilities: None,

            stdin,
            stdout,
            next_seq: 0,
        })
    }

    pub async fn quit(&mut self) -> Result<(), std::io::Error> {
        self.child.kill().await
    }

    pub async fn send_request(&mut self, request: Request) -> Result<()> {
        let seq = self.next_seq();

        self.requests.insert(seq, request.clone());

        let req = serde_json::to_string(&AdapterMessage::Request(RequestPayload { seq, request }))?;

        self.write(req).await?;

        Ok(())
    }

    pub async fn send_response(
        &mut self,
        request_seq: u32,
        success: bool,
        message: Option<String>,
        response: Response,
    ) -> Result<()> {
        let seq = self.next_seq();

        let res = serde_json::to_string(&AdapterMessage::Response(ResponsePayload {
            seq,
            request_seq,
            success,
            message,
            response,
        }))?;

        self.write(res).await?;

        Ok(())
    }

    pub fn get_request(&mut self, seq: u32) -> Option<Request> {
        self.requests.remove(&seq)
    }

    pub fn num_requests(&self) -> usize {
        self.requests.len()
    }

    pub async fn read(&mut self) -> Result<Option<AdapterMessage>> {
        // Parse headers
        let mut headers = HashMap::new();
        loop {
            let mut header = String::new();
            if self.stdout.read_line(&mut header).await? == 0 {
                debug!("Debug adapter closed pipe, stopping reading");
                return Ok(None);
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
            .ok_or_else(|| anyhow!("Failed to find Content-Length header"))?
            .parse()
            .context("Failed to parse Content-Length header")?;

        // Parse the message
        let mut content = vec![0; content_len];
        self.stdout.read_exact(&mut content).await?;
        // TODO: Remove the intermediary string
        let content = String::from_utf8(content).expect("Failed to read content as UTF-8 string");
        debug!("[DEBUG ADAPTER] >> {}", content);
        match serde_json::from_str::<AdapterMessage>(&content) {
            Ok(msg) => Ok(Some(msg)),
            Err(e) => bail!("[ADAPTER RX] {}", e),
        }
    }

    pub fn update_seq(&mut self, new_seq: u32) {
        if new_seq >= self.next_seq {
            self.next_seq = new_seq + 1
        }
    }

    async fn write(&mut self, msg: String) -> Result<()> {
        debug!("[DEBUG ADAPTER] << {}", msg);
        self.stdin
            .write_all(format!("Content-Length: {}\r\n\r\n{}", msg.len(), msg).as_bytes())
            .await?;
        self.stdin.flush().await?;

        Ok(())
    }

    fn next_seq(&mut self) -> u32 {
        let seq = self.next_seq;
        self.next_seq += 1;
        seq
    }
}
