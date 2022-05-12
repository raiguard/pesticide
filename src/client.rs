use crate::dap_types::*;
use anyhow::Result;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::select;
use tokio::sync::{broadcast, mpsc};

pub async fn run(socket_path: PathBuf) -> Result<()> {
    // Server message handling task
    let (to_server_tx, mut to_server_rx) = mpsc::channel::<String>(32);
    let (server_in_tx, mut server_in_rx) = broadcast::channel::<String>(32);
    tokio::spawn(async move {
        let server = UnixStream::connect(socket_path).await.unwrap();
        let (server_rd, mut server_wr) = tokio::io::split(server);
        let mut server_rd = BufReader::new(server_rd);
        let mut read_buf = String::new();

        loop {
            select! {
                res = to_server_rx.recv() => {
                    match res {
                        Some(mut req) => {
                            req += "\n";
                            server_wr.write_all(req.as_bytes()).await.unwrap();
                        },
                        None => break,

                    };
                }
                res = server_rd.read_line(&mut read_buf) => {
                    match res {
                        Ok(0) => {
                            println!("Server disconnected");
                            server_in_tx.send("quit".to_string()).unwrap();
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

    let mut stdin = BufReader::new(tokio::io::stdin());
    loop {
        let mut input = String::new();
        select! {
            Ok(_) = stdin.read_line(&mut input) => {
                to_server_tx.send(input).await?;
            },
            Ok(msg) = server_in_rx.recv() => {
                #[allow(clippy::single_match)]
                match msg.as_str() {
                    "quit" => break,
                    _ => ()
                }
            }
        }
    }

    Ok(())
}
