use anyhow::Result;
use crossterm::event::KeyCode;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use futures_util::{SinkExt, StreamExt};
use std::path::PathBuf;
use tokio::net::UnixStream;
use tokio::select;
use tokio_util::codec::{Framed, LinesCodec};

pub async fn run(socket_path: PathBuf) -> Result<()> {
    // Set up terminal
    enable_raw_mode()?;

    let socket = UnixStream::connect(socket_path).await.unwrap();
    let mut socket = Framed::new(socket, LinesCodec::new());
    let mut input_stream = crossterm::event::EventStream::new();

    loop {
        select! {
            // User input
            Some(Ok(event)) = input_stream.next() => handle_input(&mut socket, event).await?,
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
                    }
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

    // Restore terminal
    disable_raw_mode()?;

    Ok(())
}

async fn handle_input(
    socket: &mut Framed<UnixStream, LinesCodec>,
    event: crossterm::event::Event,
) -> Result<()> {
    match event {
        crossterm::event::Event::Key(event) => match event.code {
            KeyCode::Backspace => (),
            KeyCode::Enter => (),
            KeyCode::Left => (),
            KeyCode::Right => (),
            KeyCode::Up => (),
            KeyCode::Down => (),
            KeyCode::Home => (),
            KeyCode::End => (),
            KeyCode::PageUp => (),
            KeyCode::PageDown => (),
            KeyCode::Tab => (),
            KeyCode::BackTab => (),
            KeyCode::Delete => (),
            KeyCode::Insert => (),
            KeyCode::F(_) => (),
            KeyCode::Char(c) => match c {
                'q' => socket.send("quit".to_string()).await?,
                'i' => socket.send("in".to_string()).await?,
                _ => (),
            },
            KeyCode::Null => (),
            KeyCode::Esc => (),
        },
        crossterm::event::Event::Mouse(_) => (),
        crossterm::event::Event::Resize(_, _) => (),
    };
    println!("{:?}", event);

    Ok(())
}
