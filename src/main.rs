// LOGIC:
// Each debug session has one server and N clients
// Each session has a socket in $XDG_RUNTIME_DIR/pesticide, keyed by session name
// The session name is either provided as a CLI flag or is set to the PID
// Daemon mode will start a server without starting any clients
// Kakoune mode will start a client that is connected to the Kakoune session instead of being a standard client
// Regular clients can take arguments to specify how their UI is configured
// Request mode will construct a client, send a single request, then quit
// The server will manage a log in $HOME/.local/share/pesticide/[session].log
// Clients will send their log messages to the server so they all get written to the same file
// Socket messages will use something similar to the DAP - a content length header, then the data as JSON
//
// IMPLEMENTATION:
// Construct a server and client, and pass simple data back and forth
// Send log messages to server to have a single log file
// Connect existing adapter logic to the server
// Send updated events to clients
// Clients request the data they need
// TUI
//
// SERVER ARCHITECTURE:
// - Dedicated task to read and write the data structures
// - A separate task for each client that uses channels to send and receive data from the data task
// - Separate task to listen to server stdout, or use select! in the management task?
// - Each client task will need to be able to send events to the client when the state updates without user input
//
// CLIENT ARCHITECTURE:
// - Dedicated task to manage I/O with the server
// - select! over client TUI or CLI inputs, and messages received from the server
// - separate tasks for rendering the UI and accepting user input

use anyhow::Result;
use pico_args::Arguments;
use std::path::PathBuf;
use std::{thread, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::select;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let mut args = Arguments::from_env();
    if args.contains("--help") {
        println!("{}", HELP);
        return Ok(());
    }
    let run_type = if args.contains("--server") {
        RunType::Server
    } else {
        RunType::Client
    };
    let cli = Cli {
        session: args.opt_value_from_str("--session")?,
        run_type,
    };

    // Get socket path
    let pid = std::process::id().to_string();
    let runtime_dir = dirs::runtime_dir()
        .expect("Could not get runtime directory")
        .join("pesticide");
    if !runtime_dir.exists() {
        tokio::fs::create_dir_all(&runtime_dir).await?;
    }
    let socket_path = runtime_dir.join(format!("{}.sock", cli.session.as_ref().unwrap_or(&pid)));

    match cli.run_type {
        RunType::Client => run_client(socket_path).await,
        RunType::Server => run_server(socket_path).await,
    }
}

#[derive(Debug)]
pub struct Cli {
    run_type: RunType,
    session: Option<String>,
}

#[derive(Debug)]
enum RunType {
    Client,
    Server,
}

const HELP: &str = "\
usage: pesticide [options]
options:
    --help            Print help information
    --server          Start a headless session
    --session <NAME>  Set a session name (default: PID)
";

async fn run_client(socket_path: PathBuf) -> Result<()> {
    // Server message handling task
    let (server_out_tx, mut server_out_rx) = tokio::sync::mpsc::channel::<String>(32);
    let (server_in_tx, mut server_in_rx) = tokio::sync::broadcast::channel::<String>(32);
    tokio::spawn(async move {
        let server = UnixStream::connect(socket_path).await.unwrap();
        let (server_rd, mut server_wr) = tokio::io::split(server);
        let mut server_rd = BufReader::new(server_rd);
        let mut read_buf = String::new();

        loop {
            select! {
                res = server_out_rx.recv() => {
                    match res {
                        Some(msg) => {
                            println!("TO SERVER: {}", msg.trim());
                            server_wr.write_all(msg.as_bytes()).await.unwrap();
                        },
                        None => break,

                    };
                }
                res = server_rd.read_line(&mut read_buf) => {
                    match res {
                        Ok(0) => {
                            println!("Server disconnected");
                            break
                        },
                        Ok(_) => {
                            let msg = read_buf.trim();
                            println!("FROM SERVER: {}", msg);
                            server_in_tx.send(msg.to_string()).unwrap();
                        },
                        Err(e) => eprintln!("{}", e),
                    };
                    read_buf.clear();
                }
            }
        }
    });

    // Receive messages from the server
    tokio::spawn(async move {
        loop {
            let msg = server_in_rx.recv().await.unwrap();
            println!("RECEIVED FROM MANAGER: {}", msg);
        }
    });

    // Send dummy messages to server
    for i in 1..5 {
        server_out_tx.send(format!("Hello, world {}!\n", i)).await?;
        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}

async fn run_server(socket_path: PathBuf) -> Result<()> {
    let listener = UnixListener::bind(socket_path)?;

    loop {
        let (stream, addr) = listener.accept().await?;
        println!("NEW CLIENT: {:?}", addr);
        let (rd, mut wr) = tokio::io::split(stream);
        let mut buf = BufReader::new(rd);

        tokio::spawn(async move {
            loop {
                let mut msg = String::new();
                match buf.read_line(&mut msg).await {
                    Ok(0) => {
                        println!("Client disconnected");
                        break;
                    } // Client disconnected
                    Ok(_) => {
                        println!("FROM CLIENT: {}", msg.trim());
                        wr.write_all(msg.as_bytes()).await.unwrap();
                    }
                    Err(_) => (), // TODO:
                };
            }
        });
    }
}
