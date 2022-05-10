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

use anyhow::Result;
use pico_args::Arguments;
use std::{thread, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

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
    println!("{:?}", socket_path);

    match cli.run_type {
        // RunType::Client if cli.session.is_none() => {
        //     // Fork server to background, then run client
        //     println!("Client-server mode");
        // }
        RunType::Client => {
            let stream = UnixStream::connect(socket_path).await?;
            let (rd, mut wr) = tokio::io::split(stream);
            let mut rd = BufReader::new(rd);

            for _ in 1..10 {
                wr.write_all(b"Hello world!\n").await?;
                let mut msg = String::new();
                rd.read_line(&mut msg).await?;
                println!("{}", msg.trim());
                thread::sleep(Duration::from_secs(1));
            }
        }
        RunType::Server => {
            let listener = UnixListener::bind(socket_path)?;

            loop {
                let (stream, addr) = listener.accept().await?;
                println!("New client: {:?}", addr);
                let (rd, mut wr) = tokio::io::split(stream);
                let mut buf = BufReader::new(rd);

                tokio::spawn(async move {
                    loop {
                        let mut msg = String::new();
                        match buf.read_line(&mut msg).await {
                            Ok(0) => break, // Client disconnected
                            Ok(_) => {
                                println!("{}", msg.trim());
                                wr.write_all(b"RECEIVEWD\n").await.unwrap();
                            }
                            Err(_) => (), // TODO:
                        };
                    }
                });
            }
        }
    }

    Ok(())
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
