use crate::config::Config;
use crate::dap::codec::DAPCodec;
use crate::dap::*;
use anyhow::{Context, Result};
use futures_util::SinkExt;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio_util::codec::{FramedRead, FramedWrite};

pub struct Adapter {
    pub child: Child,
    pub config: Config,

    /// Capabilities defined by the adapter
    /// TODO: Wait to construct adapter object until after this is retrieved?
    pub capabilities: Option<Capabilities>,
    /// Responses from the debug adapter will use the seq as an identifier
    requests: HashMap<u32, RequestArguments>,

    stdin: FramedWrite<ChildStdin, DAPCodec>,
    pub stdout: FramedRead<ChildStdout, DAPCodec>,
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

        let stdout = FramedRead::new(
            child.stdout.take().context("Failed to open stdout")?,
            DAPCodec::new(),
        );
        let stdin = FramedWrite::new(
            child.stdin.take().context("Failed to open stdin")?,
            DAPCodec::new(),
        );

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

    pub async fn send_request(&mut self, request: RequestArguments) -> Result<()> {
        let seq = self.next_seq();

        self.requests.insert(seq, request.clone());

        self.write(ProtocolMessage {
            seq,
            type_: ProtocolMessageType::Request(request),
        })
        .await?;

        Ok(())
    }

    pub async fn send_response(
        &mut self,
        request_seq: u32,
        success: bool,
        response: ResponseBody,
    ) -> Result<()> {
        let seq = self.next_seq();

        self.write(ProtocolMessage {
            seq,
            type_: ProtocolMessageType::Response(Response {
                request_seq,
                success,
                result: ResponseResult::Success { body: response },
            }),
        })
        .await?;

        Ok(())
    }

    pub fn get_request(&mut self, seq: u32) -> Option<RequestArguments> {
        self.requests.remove(&seq)
    }

    pub fn num_requests(&self) -> usize {
        self.requests.len()
    }

    pub fn update_seq(&mut self, new_seq: u32) {
        if new_seq >= self.next_seq {
            self.next_seq = new_seq + 1
        }
    }

    async fn write(&mut self, msg: ProtocolMessage) -> Result<()> {
        debug!(
            "[DEBUG ADAPTER] << {}",
            serde_json::to_string(&msg).unwrap()
        );
        self.stdin.send(msg).await?;

        Ok(())
    }

    fn next_seq(&mut self) -> u32 {
        let seq = self.next_seq;
        self.next_seq += 1;
        seq
    }
}
