use crate::adapter::Adapter;
use crate::config::Config;
use crate::dap_types::*;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::select;
use tokio::sync::{mpsc, Mutex};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};

// LIFECYCLE:
// -

pub async fn run(socket_path: PathBuf, config_path: PathBuf) -> Result<()> {
    // Spin up debug adapter
    let config = Config::new(config_path)?;
    let mut adapter = Adapter::new(config)?;
    // Channel to send and receive adapter comms to/from clients
    let (adapter_tx, mut adapter_rx) = mpsc::channel::<Request>(32);

    // State is shared between all threads
    let state = Arc::new(Mutex::new(State::new()));

    // Client listener
    let client_listener = UnixListener::bind(socket_path)?;

    // Send initialize request
    let adapter_id = adapter.config.adapter_id.clone();
    adapter
        .send_request(Request::Initialize(InitializeArgs {
            client_id: Some("pesticide".to_string()),
            client_name: Some("Pesticide".to_string()),
            adapter_id,
            locale: Some("en-US".to_string()),
            lines_start_at_1: true,
            columns_start_at_1: true,
            path_format: Some(InitializePathFormat::Path),
            supports_variable_type: false,
            supports_variable_paging: false,
            supports_run_in_terminal_request: true,
            supports_memory_references: false,
            supports_progress_reporting: false,
            supports_invalidated_event: false,
            supports_memory_event: false,
        }))
        .await?;

    loop {
        select! {
            // New client connections
            Ok((stream, addr)) = client_listener.accept() => {
                let state = state.clone();
                println!("NEW CLIENT: {:?}", addr);
                let adapter_tx = adapter_tx.clone();

                tokio::spawn(async move {
                    let stream = Framed::new(stream, LinesCodec::new());

                    // Save client to shared state
                    let mut client = Client::new(state, stream).await.unwrap();

                    while let Some(line) = client.stream.next().await {
                        match line {
                            Ok(msg) => {
                                // TEMPORARY: Assume that what is being sent is an adapter request
                                let req = serde_json::from_str::<Request>(&msg).unwrap();
                                adapter_tx.send(req).await.unwrap();
                            }
                            Err(e) => error!("{}", e),
                        }
                    }
                });
            },
            // Incoming debug adapter messages
            res = adapter.read() => {
                match res {
                    Ok(Some(msg)) => {
                        // TODO:
                    }
                    Ok(None) => break,
                    Err(e) => error!("{}", e)
                }
            }
            // Requests for debug adapter
            Some(req) = adapter_rx.recv() => {
                adapter.send_request(req).await.unwrap();
            }
        }
    }

    Ok(())
}

struct State {
    /// Send messages to clients
    clients: HashMap<u32, mpsc::Sender<String>>,
    next_client: u32,

    // Debug session state
    threads: Vec<Thread>,
}

impl State {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            next_client: 0,
            threads: vec![],
        }
    }
}

struct Client {
    rx: mpsc::Receiver<String>,
    stream: Framed<UnixStream, LinesCodec>,
}

impl Client {
    pub async fn new(
        state: Arc<Mutex<State>>,
        stream: Framed<UnixStream, LinesCodec>,
    ) -> Result<Self> {
        // Create a channel for this client
        let (tx, rx) = mpsc::channel(32);

        let mut state = state.lock().await;
        let client_id = state.next_client;
        state.next_client += 1;
        state.clients.insert(client_id, tx);

        Ok(Self { rx, stream })
    }
}
