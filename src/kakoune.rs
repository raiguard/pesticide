use crate::controller::State;
use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::process::Command;

pub struct Kakoune {
    session: String,
    socket: UnixListener,
    sock_path: PathBuf,
    current_jump: Option<(String, i64)>,
}

impl Kakoune {
    pub async fn new(session: String, sock_path: PathBuf) -> Result<Self> {
        let socket = UnixListener::bind(&sock_path)?;
        Ok(Self {
            session,
            socket,
            sock_path,
            current_jump: None,
        })
    }

    pub async fn quit(&mut self, state: &mut State) -> Result<()> {
        self.clear_jump().await?;
        state.breakpoints.clear();
        self.update_breakpoints(state).await?;
        fs::remove_file(&self.sock_path).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<KakRequest> {
        let (mut connection, _) = self.socket.accept().await?;
        let mut req = String::new();
        connection.read_to_string(&mut req).await?;
        debug!("--> {req}");
        Ok(serde_json::from_str(&req)?)
    }

    pub async fn send(&mut self, command: String) -> Result<()> {
        debug!("<-- {}", command);
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
            .write_all(command.as_bytes())
            .await?;

        Ok(())
    }

    pub async fn clear_jump(&mut self) -> Result<()> {
        if let Some((path, _)) = &self.current_jump {
            self.send(format!(
                "evaluate-commands %{{
                    edit {path}
                    set-option buffer step_indicator %val{{timestamp}}
                }}"
            ))
            .await?;
            self.current_jump = None;
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
        if self.current_jump.is_some() && self.current_jump.as_ref().unwrap().0 != source_path {
            self.clear_jump().await?;
        }
        self.current_jump = Some((source_path.clone(), frame.line));
        self.update_breakpoints(state).await?;
        self.send(format!(
            r#"evaluate-commands -try-client %opt{{jumpclient}} %{{
                edit {0} {1}
                set-option buffer step_indicator %val{{timestamp}} "{1}|{{StepIndicator}}%opt{{step_symbol}}"
            }}"#,
            source_path, frame.line,
        )).await?;

        Ok(())
    }

    pub async fn update_breakpoints(&mut self, state: &State) -> Result<()> {
        // Clear all breakpoints
        let mut cmd = String::from(
            r#"evaluate-commands %sh{
                eval set -- "$kak_quoted_buflist"
                while [ $# -gt 0 ]; do
                    echo "
                        edit $1
                        set-option buffer breakpoints %val{timestamp}
                    "
                    shift
                done
            }"#,
        );
        // Set current breakpoints
        for (path, breakpoints) in &state.breakpoints {
            cmd = format!(
                // TODO: Use 'buffer' instead of 'edit' to avoid opening extra files
                "{cmd}
                try %{{
                    edit {path}
                    set-option buffer breakpoints %val{{timestamp}} {}
                }}",
                breakpoints
                    .iter()
                    .map(|breakpoint| format!(
                        r#""{}|{{Breakpoint}}%opt{{breakpoint_symbol}}" "#,
                        breakpoint.line
                    ))
                    .collect::<String>(),
            );
        }
        self.send(cmd).await?;
        Ok(())
    }
}

/// A request sent from kakoune, deserialized from JSON
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd")]
#[serde(rename_all = "snake_case")]
pub enum KakRequest {
    ToggleBreakpoint {
        file: String,
        line: i64,
        column: i64,
    },
}
