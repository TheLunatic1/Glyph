use anyhow::{anyhow, Context, Result};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SB: u8 = 250;
const SE: u8 = 240;

const OPT_ECHO: u8 = 1;
const OPT_SGA: u8 = 3;
const OPT_NAWS: u8 = 31;

#[derive(Debug, Clone)]
pub enum TelnetEvent {
    Data(Vec<u8>),
    Closed(String),
}

enum TelnetCmd {
    Data(Vec<u8>),
    Resize(u16, u16),
    Close,
}

#[derive(Clone)]
pub struct TelnetHandle {
    cmd_tx: mpsc::UnboundedSender<TelnetCmd>,
}

impl TelnetHandle {
    pub fn write(&self, bytes: Vec<u8>) {
        let _ = self.cmd_tx.send(TelnetCmd::Data(bytes));
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self.cmd_tx.send(TelnetCmd::Resize(cols.max(10), rows.max(4)));
    }

    pub fn close(&self) {
        let _ = self.cmd_tx.send(TelnetCmd::Close);
    }
}

pub struct TelnetSession;

impl TelnetSession {
    pub async fn connect(
        host: &str,
        port: u16,
        cols: u16,
        rows: u16,
    ) -> Result<(TelnetHandle, mpsc::UnboundedReceiver<TelnetEvent>)> {
        let trimmed = host.trim();
        if trimmed.is_empty() || port == 0 {
            return Err(anyhow!("Telnet host and non-zero port are required"));
        }

        let stream = tokio::time::timeout(
            Duration::from_secs(10),
            TcpStream::connect((trimmed, port)),
        )
        .await
        .map_err(|_| anyhow!("Telnet connection timed out to {trimmed}:{port}"))?
        .with_context(|| format!("Failed to connect via Telnet to {trimmed}:{port}"))?;

        let (mut rd, mut wr) = stream.into_split();
        let (ev_tx, ev_rx) = mpsc::unbounded_channel();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<TelnetCmd>();

        tokio::spawn(async move {
            let mut parser = IacParser::new(cols.max(10), rows.max(4));
            let mut buf = [0u8; 4096];
            let close_reason: String;

            loop {
                tokio::select! {
                    biased;
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(TelnetCmd::Data(bytes)) => {
                                let escaped = escape_iac(&bytes);
                                if wr.write_all(&escaped).await.is_err() {
                                    close_reason = "Telnet write error".to_string();
                                    break;
                                }
                            }
                            Some(TelnetCmd::Resize(c, r)) => {
                                parser.cols = c;
                                parser.rows = r;
                                if parser.naws_enabled {
                                    let pkt = build_naws(c, r);
                                    let _ = wr.write_all(&pkt).await;
                                }
                            }
                            Some(TelnetCmd::Close) | None => {
                                close_reason = "Telnet disconnected".to_string();
                                break;
                            }
                        }
                    }
                    read_res = rd.read(&mut buf) => {
                        match read_res {
                            Ok(0) => {
                                close_reason = "Connection closed by remote host".to_string();
                                break;
                            }
                            Ok(n) => {
                                let (display, replies) = parser.feed(&buf[..n]);
                                if !replies.is_empty() && wr.write_all(&replies).await.is_err() {
                                    close_reason = "Telnet negotiation failed".to_string();
                                    break;
                                }
                                if !display.is_empty() && ev_tx.send(TelnetEvent::Data(display)).is_err() {
                                    close_reason = "UI closed".to_string();
                                    break;
                                }
                            }
                            Err(e) => {
                                close_reason = format!("Telnet read error: {e}");
                                break;
                            }
                        }
                    }
                }
            }

            let _ = ev_tx.send(TelnetEvent::Closed(close_reason));
        });

        Ok((TelnetHandle { cmd_tx }, ev_rx))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParseState {
    Normal,
    Iac,
    Negotiate(u8),
    Subnegotiate,
    SubnegotiateIac,
}

struct IacParser {
    state: ParseState,
    cols: u16,
    rows: u16,
    naws_enabled: bool,
}

impl IacParser {
    fn new(cols: u16, rows: u16) -> Self {
        Self {
            state: ParseState::Normal,
            cols,
            rows,
            naws_enabled: false,
        }
    }

    fn feed(&mut self, input: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let mut display = Vec::with_capacity(input.len());
        let mut replies = Vec::new();

        for &b in input {
            match self.state {
                ParseState::Normal => {
                    if b == IAC {
                        self.state = ParseState::Iac;
                    } else {
                        display.push(b);
                    }
                }
                ParseState::Iac => match b {
                    IAC => {
                        display.push(IAC);
                        self.state = ParseState::Normal;
                    }
                    DO | DONT | WILL | WONT => {
                        self.state = ParseState::Negotiate(b);
                    }
                    SB => {
                        self.state = ParseState::Subnegotiate;
                    }
                    _ => {
                        self.state = ParseState::Normal;
                    }
                },
                ParseState::Negotiate(cmd) => {
                    match (cmd, b) {
                        (DO, OPT_NAWS) => {
                            self.naws_enabled = true;
                            replies.extend_from_slice(&[IAC, WILL, OPT_NAWS]);
                            replies.extend_from_slice(&build_naws(self.cols, self.rows));
                        }
                        (DO, OPT_SGA) => {
                            replies.extend_from_slice(&[IAC, WILL, OPT_SGA]);
                        }
                        (DO, opt) => {
                            replies.extend_from_slice(&[IAC, WONT, opt]);
                        }
                        (WILL, OPT_ECHO | OPT_SGA) => {
                            replies.extend_from_slice(&[IAC, DO, b]);
                        }
                        (WILL, opt) => {
                            replies.extend_from_slice(&[IAC, DONT, opt]);
                        }
                        (DONT, opt) => {
                            if opt == OPT_NAWS {
                                self.naws_enabled = false;
                            }
                            replies.extend_from_slice(&[IAC, WONT, opt]);
                        }
                        (WONT, opt) => {
                            replies.extend_from_slice(&[IAC, DONT, opt]);
                        }
                        _ => {}
                    }
                    self.state = ParseState::Normal;
                }
                ParseState::Subnegotiate => {
                    if b == IAC {
                        self.state = ParseState::SubnegotiateIac;
                    }
                }
                ParseState::SubnegotiateIac => {
                    if b == SE {
                        self.state = ParseState::Normal;
                    } else if b != IAC {
                        self.state = ParseState::Subnegotiate;
                    }
                }
            }
        }

        (display, replies)
    }
}

fn build_naws(cols: u16, rows: u16) -> Vec<u8> {
    let mut out = vec![IAC, SB, OPT_NAWS];
    for byte in cols
        .to_be_bytes()
        .into_iter()
        .chain(rows.to_be_bytes())
    {
        if byte == IAC {
            out.push(IAC);
        }
        out.push(byte);
    }
    out.push(IAC);
    out.push(SE);
    out
}

fn escape_iac(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for &b in bytes {
        out.push(b);
        if b == IAC {
            out.push(IAC);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn telnet_loopback_iac_naws_and_echo_e2e() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();

        let srv_task = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            // Send IAC DO NAWS + IAC WILL ECHO + literal 0xFF (IAC IAC) + welcome
            sock.write_all(&[IAC, DO, OPT_NAWS, IAC, WILL, OPT_ECHO, b'O', b'K', IAC, IAC])
                .await
                .unwrap();
            let mut in_buf = vec![0u8; 64];
            let mut got = Vec::new();
            let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
            while tokio::time::Instant::now() < deadline && !got.windows(4).any(|w| w == b"ping") {
                if let Ok(Ok(n)) = tokio::time::timeout(Duration::from_millis(300), sock.read(&mut in_buf)).await {
                    if n == 0 {
                        break;
                    }
                    got.extend_from_slice(&in_buf[..n]);
                }
            }
            got
        });

        let (handle, mut rx) = TelnetSession::connect("127.0.0.1", addr.port(), 100, 30)
            .await
            .unwrap();

        let mut received = Vec::new();
        while let Ok(Some(TelnetEvent::Data(chunk))) =
            tokio::time::timeout(Duration::from_secs(2), rx.recv()).await
        {
            received.extend_from_slice(&chunk);
            if received == [b'O', b'K', 0xFF] {
                break;
            }
        }
        assert_eq!(received, vec![b'O', b'K', 0xFF]);

        handle.write(b"ping\r\n".to_vec());
        let srv_got = srv_task.await.unwrap();
        handle.close();

        // Server must have received WILL NAWS + NAWS SB + DO ECHO + "ping\r\n"
        assert!(srv_got.windows(3).any(|w| w == [IAC, WILL, OPT_NAWS]));
        assert!(srv_got.windows(3).any(|w| w == [IAC, DO, OPT_ECHO]));
        assert!(srv_got.windows(4).any(|w| w == b"ping"));
    }
}
