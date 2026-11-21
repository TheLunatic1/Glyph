use anyhow::{anyhow, Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc as std_mpsc;
use std::thread;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum LocalPtyEvent {
    Data(Vec<u8>),
    Closed(Option<String>),
}

enum LocalPtyCmd {
    Data(Vec<u8>),
    Resize(u16, u16),
    Close,
}

#[derive(Clone)]
pub struct LocalPtyHandle {
    cmd_tx: std_mpsc::Sender<LocalPtyCmd>,
}

impl LocalPtyHandle {
    pub fn write(&self, bytes: Vec<u8>) {
        let _ = self.cmd_tx.send(LocalPtyCmd::Data(bytes));
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self.cmd_tx.send(LocalPtyCmd::Resize(cols.max(10), rows.max(4)));
    }

    pub fn close(&self) {
        let _ = self.cmd_tx.send(LocalPtyCmd::Close);
    }
}

pub fn default_local_shell() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        if which_exists("pwsh.exe") {
            vec!["pwsh.exe".to_string(), "-NoLogo".to_string()]
        } else if which_exists("powershell.exe") {
            vec!["powershell.exe".to_string(), "-NoLogo".to_string()]
        } else {
            vec![std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())]
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if std::path::Path::new("/bin/bash").exists() {
                "/bin/bash".to_string()
            } else {
                "/bin/sh".to_string()
            }
        });
        vec![shell]
    }
}

#[cfg(target_os = "windows")]
fn which_exists(bin: &str) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            if dir.join(bin).is_file() {
                return true;
            }
        }
    }
    false
}

pub struct LocalPtySession;

impl LocalPtySession {
    pub fn spawn(
        cols: u16,
        rows: u16,
        command_override: Option<Vec<String>>,
    ) -> Result<(LocalPtyHandle, mpsc::UnboundedReceiver<LocalPtyEvent>)> {
        let argv = command_override
            .filter(|v| !v.is_empty())
            .unwrap_or_else(default_local_shell);

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: rows.max(4),
                cols: cols.max(10),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| anyhow!("Failed to open local PTY: {e}"))?;

        let mut cmd = CommandBuilder::new(&argv[0]);
        for arg in &argv[1..] {
            cmd.arg(arg);
        }
        if let Some(base) = directories::BaseDirs::new() {
            cmd.cwd(base.home_dir());
        }
        cmd.env("TERM", "xterm-256color");

        let mut child = pair
            .slave
            .spawn_command(cmd)
            .with_context(|| format!("Failed to launch local shell \"{}\"", argv[0]))?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| anyhow!("Failed to clone PTY reader: {e}"))?;
        let mut writer = pair
            .master
            .take_writer()
            .map_err(|e| anyhow!("Failed to take PTY writer: {e}"))?;
        let master = pair.master;

        let (ev_tx, ev_rx) = mpsc::unbounded_channel();
        let (cmd_tx, cmd_rx) = std_mpsc::channel::<LocalPtyCmd>();

        let ev_read = ev_tx.clone();
        thread::Builder::new()
            .name("glyph-pty-reader".to_string())
            .spawn(move || {
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if ev_read.send(LocalPtyEvent::Data(buf[..n].to_vec())).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let _ = ev_read.send(LocalPtyEvent::Closed(None));
            })
            .ok();

        thread::Builder::new()
            .name("glyph-pty-writer".to_string())
            .spawn(move || {
                while let Ok(cmd) = cmd_rx.recv() {
                    match cmd {
                        LocalPtyCmd::Data(bytes) => {
                            if writer.write_all(&bytes).is_err() || writer.flush().is_err() {
                                break;
                            }
                        }
                        LocalPtyCmd::Resize(c, r) => {
                            let _ = master.resize(PtySize {
                                rows: r,
                                cols: c,
                                pixel_width: 0,
                                pixel_height: 0,
                            });
                        }
                        LocalPtyCmd::Close => {
                            let _ = child.kill();
                            break;
                        }
                    }
                }
                let _ = child.kill();
                let _ = child.wait();
                let _ = ev_tx.send(LocalPtyEvent::Closed(None));
            })
            .ok();

        Ok((LocalPtyHandle { cmd_tx }, ev_rx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn local_pty_spawns_and_captures_output() {
        #[cfg(target_os = "windows")]
        let cmd = vec![
            "cmd.exe".to_string(),
            "/C".to_string(),
            "echo GLYPH_LOCAL_PTY_OK".to_string(),
        ];
        #[cfg(not(target_os = "windows"))]
        let cmd = vec![
            "/bin/sh".to_string(),
            "-c".to_string(),
            "echo GLYPH_LOCAL_PTY_OK".to_string(),
        ];

        let (handle, mut rx) = LocalPtySession::spawn(80, 24, Some(cmd)).unwrap();
        let mut collected = String::new();
        let mut replied_dsr = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(8);

        while tokio::time::Instant::now() < deadline && !collected.contains("GLYPH_LOCAL_PTY_OK") {
            match tokio::time::timeout(Duration::from_millis(300), rx.recv()).await {
                Ok(Some(LocalPtyEvent::Data(bytes))) => {
                    collected.push_str(&String::from_utf8_lossy(&bytes));
                    if !replied_dsr && collected.contains("\x1b[6n") {
                        replied_dsr = true;
                        handle.write(b"\x1b[1;1R".to_vec());
                    }
                }
                Ok(Some(LocalPtyEvent::Closed(_))) | Ok(None) => break,
                Err(_) => {}
            }
        }
        handle.close();
        assert!(
            collected.contains("GLYPH_LOCAL_PTY_OK"),
            "Unexpected local PTY output: {collected:?}"
        );
    }
}
