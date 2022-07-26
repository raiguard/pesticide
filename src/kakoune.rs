use anyhow::Result;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::process::Command;

pub struct Kakoune {
    session: String,
    socket: UnixListener,
    sock_path: PathBuf,
}

impl Kakoune {
    pub async fn new(session: String, sock_path: PathBuf) -> Result<Self> {
        let socket = UnixListener::bind(&sock_path)?;
        Ok(Self {
            session,
            socket,
            sock_path,
        })
    }

    pub async fn quit(&mut self) -> Result<(), io::Error> {
        fs::remove_file(&self.sock_path).await
    }

    pub async fn listen(&mut self) -> Result<String> {
        let (mut connection, _) = self.socket.accept().await?;
        let mut req = String::new();
        connection.read_to_string(&mut req).await?;
        debug!("--> {req}");
        Ok(req)
    }

    pub async fn send(&mut self, command: KakCmd) -> Result<()> {
        let cmd = match command {
            KakCmd::Jump { file, line, column } => {
                format!(
                    "evaluate-commands -try-client %opt{{jumpclient}} %{{
                        edit {0} {1} {2}
                        set-option buffer pesticide_flags %val{{timestamp}} \"{1}|{{StepIndicator}}%opt{{step_symbol}}\"
                        add-highlighter -override buffer/step_line line {1} StepLine
                    }}",
                    file, line, column.unwrap_or(1)
                )
            }
        };

        debug!("<-- {}", cmd);

        // 'kak -p' will not execute until the pipe is closed, so we must spawn a new one every time...
        Command::new("kak")
            .stderr(Stdio::null())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .arg("-p")
            .arg(&self.session)
            .spawn()?
            .stdin
            .take()
            .unwrap()
            .write_all(cmd.as_bytes())
            .await?;

        Ok(())
    }
}

pub enum KakCmd {
    Jump {
        file: String,
        line: i64,
        column: Option<i64>,
    },
}
