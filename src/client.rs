use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tokio::select;
use tokio_util::codec::{Framed, LinesCodec};

pub async fn run(socket_path: PathBuf) -> Result<()> {
    let socket = UnixStream::connect(socket_path).await.unwrap();
    let mut socket = Framed::new(socket, LinesCodec::new());
    let mut stdin = BufReader::new(tokio::io::stdin());

    loop {
        trace!("New loop");
        let mut input = String::new();
        select! {
            // User input
            Ok(_) = stdin.read_line(&mut input) => {
                let input = input + "\n";
                socket.send(input).await?;
            },
            // Messages from server
            msg = socket.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        debug!("FROM SERVER: {msg}");
                        #[allow(clippy::single_match)]
                        match msg.as_str() {
                            "quit" => break,
                            _ => ()
                        }
                    },
                    Some(Err(e)) => {
                        error!("Socket error: {e}");
                    }
                    None => {
                        info!("Server disconnected");
                        break
                    }
                }
            }
        }
    }

    trace!("End of loop");
    Ok(())
}
