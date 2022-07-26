use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::process::Command;

use crate::controller::State;

pub struct Kakoune {
    session: String,
    socket: UnixListener,
    sock_path: PathBuf,
    jump_buffer: Option<String>,
}

impl Kakoune {
    pub async fn new(session: String, sock_path: PathBuf) -> Result<Self> {
        let socket = UnixListener::bind(&sock_path)?;
        Ok(Self {
            session,
            socket,
            sock_path,
            jump_buffer: None,
        })
    }

    pub async fn quit(&mut self) -> Result<()> {
        if let Some(jump_buffer) = &self.jump_buffer {
            self.send(KakCmd::ClearJump(jump_buffer.clone())).await?;
        }
        fs::remove_file(&self.sock_path).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<KakCmd> {
        let (mut connection, _) = self.socket.accept().await?;
        let mut req = String::new();
        connection.read_to_string(&mut req).await?;
        debug!("--> {req}");
        Ok(serde_json::from_str(&req)?)
    }

    pub async fn send(&mut self, command: KakCmd) -> Result<()> {
        if let Some(cmd) = match command {
            KakCmd::ClearJump(file) => {
                Some(format!(
                    "evaluate-commands %{{
                        edit {0}
                        set-option buffer pesticide_flags %val{{timestamp}}
                        remove-highlighter buffer/step_line
                    }}",
                    file
                ))
            }
            KakCmd::Jump { file, line, column } => {
                Some(format!(
                    "evaluate-commands -try-client %opt{{jumpclient}} %{{
                        edit {0} {1} {2}
                        set-option buffer pesticide_flags %val{{timestamp}} \"{1}|{{StepIndicator}}%opt{{step_symbol}}\"
                        add-highlighter -override buffer/step_line line {1} StepLine
                    }}",
                    file, line, column.unwrap_or(1)
                ))
            }
            KakCmd::UpdateFlags => {
                todo!()
            }
            // Not all KakCmd's are sent to the editor
            _ => None
        } {
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
        }

        Ok(())
    }

    pub async fn jump(&mut self, state: &State) -> Result<()> {
        let frames = state.stack_frames.get(&state.current_thread).unwrap();
        let frame = frames
            .iter()
            .find(|frame| frame.id == state.current_stack_frame)
            .unwrap();
        let source = frame.source.as_ref().unwrap();
        let source_path = source.path.clone().unwrap();
        if let Some(jump_buffer) = &self.jump_buffer {
            if source_path != *jump_buffer {
                self.clear_jump().await?;
            }
        }
        self.jump_buffer = Some(source_path.clone());
        self.send(KakCmd::Jump {
            file: source_path,
            line: frame.line,
            column: Some(frame.column),
        })
        .await?;

        Ok(())
    }

    pub async fn clear_jump(&mut self) -> Result<()> {
        if let Some(jump_buffer) = &self.jump_buffer {
            self.send(KakCmd::ClearJump(jump_buffer.clone())).await?;
            self.jump_buffer = None;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd")]
#[serde(rename_all = "snake_case")]
pub enum KakCmd {
    ClearJump(String),
    Jump {
        file: String,
        line: i64,
        column: Option<i64>,
    },
    ToggleBreakpoint {
        file: String,
        line: i64,
        column: i64,
    },
    UpdateFlags,
}
