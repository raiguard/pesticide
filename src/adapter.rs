use crate::config::Config;
use crate::dap_types::*;
use anyhow::{bail, Context, Result};
use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread;

pub struct Adapter {
    pub child: Child,
    pub config: Config,
    pub rx: Receiver<AdapterMessage>,
    // pub tx: Sender<AdapterMessage>,
    pub next_seq: u32,

    pub capabilities: Option<Capabilities>,
    pub threads: HashMap<u32, Thread>,
    pub stack_frames: HashMap<u32, Vec<StackFrame>>,
    pub scopes: HashMap<u32, Vec<Scope>>,

    /// Responses from the debug adapter will use the seq as an identifier
    requests: HashMap<u32, Request>,

    stdin: BufWriter<ChildStdin>,
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
        thread::spawn(move || loop {
            let mut buf = String::new();
            stderr
                .read_to_string(&mut buf)
                .context("Failed to read stderr")
                .unwrap();
            if buf.is_empty() {
                continue;
            }
            error!("[DEBUG ADAPTER] >> {}", buf);
        });

        let stdout = BufReader::new(child.stdout.take().context("Failed to open stdout")?);
        let (out_tx, out_rx) = crossbeam_channel::bounded(1024);
        thread::spawn(move || {
            reader_loop(stdout, &out_tx).expect("Failed to read message from debug adapter");
        });

        let stdin = BufWriter::new(child.stdin.take().context("Failed to open stdin")?);
        // let (in_tx, in_rx) = crossbeam_channel::bounded(1024);
        // thread::spawn(move || {
        //     writer_loop(stdin, &in_rx).expect("Failed to read message from debug adapter");
        // });

        Ok(Self {
            child,
            config,
            rx: out_rx,
            next_seq: 0,

            capabilities: None,
            threads: HashMap::new(),
            stack_frames: HashMap::new(),
            scopes: HashMap::new(),

            requests: HashMap::new(),
            stdin,
        })
    }

    pub fn next_seq(&mut self) -> u32 {
        let seq = self.next_seq;
        self.next_seq += 1;
        seq
    }

    pub fn update_seq(&mut self, new_seq: u32) {
        if new_seq >= self.next_seq {
            self.next_seq = new_seq + 1
        }
    }

    pub fn send_request(&mut self, request: Request) -> Result<()> {
        let seq = self.next_seq();

        self.requests.insert(seq, request.clone());

        let req = serde_json::to_string(&AdapterMessage::Request(RequestPayload { seq, request }))?;

        self.write(req)?;

        Ok(())
    }

    pub fn send_response(
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

        self.write(res)?;

        Ok(())
    }

    pub fn get_request(&mut self, seq: u32) -> Option<Request> {
        self.requests.remove(&seq)
    }

    fn write(&mut self, msg: String) -> Result<()> {
        debug!("[DEBUG ADAPTER] << {}", msg);
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", msg.len(), msg)?;
        self.stdin.flush()?;

        Ok(())
    }
}

fn reader_loop(mut reader: impl BufRead, tx: &Sender<AdapterMessage>) -> Result<()> {
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

        let mut content = vec![0; content_len];
        reader.read_exact(&mut content)?;
        let content = String::from_utf8(content).expect("Failed to read content as UTF-8 string");
        debug!("[DEBUG ADAPTER] >> {}", content);

        match serde_json::from_str::<AdapterMessage>(&content) {
            Ok(msg) => tx
                .send(msg)
                .expect("Failed to send message from debug adapter"),
            Err(e) => error!("[ADAPTER RX] {}", e),
        }
    }
}
