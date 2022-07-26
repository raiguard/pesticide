use anyhow::Result;
use std::process::Stdio;
use tokio::{io::AsyncWriteExt, process::Command};

pub struct Kakoune {
    session: String,
}

impl Kakoune {
    pub fn new(session: String) -> Result<Self> {
        Ok(Self { session })
    }

    pub async fn exit(&mut self) -> Result<()> {
        // self.child.kill().await?;
        Ok(())
    }

    pub async fn send(&mut self, command: KakCmd) -> Result<()> {
        let cmd = match command {
            KakCmd::Jump { file, line, column } => {
                format!(
                    "evaluate-commands -try-client %opt{{jumpclient}} %{{
                        edit {0} {1} {2}
                        set-option buffer pesticide_flags %val{{timestamp}} \"{1}|{{StepIndicator}}%opt{{step_symbol}}\"
                    }}",
                    file, line, column.unwrap_or(1)
                )
            }
        };

        debug!("<-- {}", cmd);

        // 'kak -p' will not execute until the pipe is closed, so we must spawn a new one every time...
        Command::new("kak")
            .stdin(Stdio::piped())
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
