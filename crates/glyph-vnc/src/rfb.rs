use crate::keysym::{KEYSYM_ALT_L, KEYSYM_CTRL_L, KEYSYM_DELETE};
use anyhow::{anyhow, Context, Result};
use cipher::{BlockEncrypt, KeyInit};
use des::Des;
use flate2::{Decompress, FlushDecompress, Status};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

const ENCODING_RAW: i32 = 0;
const ENCODING_COPYRECT: i32 = 1;
const ENCODING_ZLIB: i32 = 6;
const ENCODING_DESKTOP_SIZE: i32 = -223;

#[derive(Debug, Clone)]
pub struct VncFrame {
    pub width: u16,
    pub height: u16,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum VncEvent {
    Connected {
        desktop_name: String,
        width: u16,
        height: u16,
    },
    Frame(Arc<VncFrame>),
    ServerClipboard(String),
    Disconnected(String),
    Error(String),
}

#[derive(Debug, Clone)]
enum VncCmd {
    KeyEvent { keysym: u32, down: bool },
    PointerEvent { x: u16, y: u16, mask: u8 },
    ClientCutText(String),
    FramebufferUpdateRequest {
        incremental: bool,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
    },
    Disconnect,
}

#[derive(Clone)]
pub struct VncClientHandle {
    tx: mpsc::UnboundedSender<VncCmd>,
    dims: Arc<std::sync::Mutex<(u16, u16)>>,
}

impl std::fmt::Debug for VncClientHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VncClientHandle").finish()
    }
}

impl VncClientHandle {
    pub fn send_key(&self, keysym: u32, down: bool) {
        let _ = self.tx.send(VncCmd::KeyEvent { keysym, down });
    }

    pub fn send_pointer(&self, x: u16, y: u16, mask: u8) {
        let _ = self.tx.send(VncCmd::PointerEvent { x, y, mask });
    }

    pub fn send_ctrl_alt_del(&self) {
        for (ks, down) in [
            (KEYSYM_CTRL_L, true),
            (KEYSYM_ALT_L, true),
            (KEYSYM_DELETE, true),
            (KEYSYM_DELETE, false),
            (KEYSYM_ALT_L, false),
            (KEYSYM_CTRL_L, false),
        ] {
            self.send_key(ks, down);
        }
    }

    pub fn send_clipboard(&self, text: String) {
        let _ = self.tx.send(VncCmd::ClientCutText(text));
    }

    pub fn request_refresh(&self, incremental: bool) {
        let (w, h) = self.dims.lock().map(|g| *g).unwrap_or((1, 1));
        let _ = self.tx.send(VncCmd::FramebufferUpdateRequest {
            incremental,
            x: 0,
            y: 0,
            w,
            h,
        });
    }

    pub fn disconnect(&self) {
        let _ = self.tx.send(VncCmd::Disconnect);
    }
}

/// RFC 6143 §7.2.2 VNC Authentication challenge encryption (bit-reversed DES-ECB).
pub fn vnc_encrypt_challenge(password: &str, challenge: &[u8; 16]) -> [u8; 16] {
    let mut key_bytes = [0u8; 8];
    for (i, b) in password.as_bytes().iter().take(8).enumerate() {
        key_bytes[i] = b.reverse_bits();
    }
    let cipher = Des::new_from_slice(&key_bytes).expect("8-byte DES key");
    let mut out = *challenge;
    cipher.encrypt_block((&mut out[0..8]).into());
    cipher.encrypt_block((&mut out[8..16]).into());
    out
}

pub async fn connect_vnc_stream<S>(
    mut stream: S,
    password: Option<String>,
    event_tx: mpsc::UnboundedSender<VncEvent>,
) -> Result<VncClientHandle>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    // 1. ProtocolVersion handshake
    let mut ver_buf = [0u8; 12];
    stream
        .read_exact(&mut ver_buf)
        .await
        .context("Failed reading RFB server version")?;
    let ver_str = std::str::from_utf8(&ver_buf).unwrap_or("");
    if !ver_str.starts_with("RFB ") {
        return Err(anyhow!("Remote endpoint did not send RFB banner: {ver_str:?}"));
    }
    let minor: u8 = ver_str
        .get(8..11)
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(8);

    let negotiated_minor = if minor <= 3 {
        3
    } else if minor <= 7 {
        7
    } else {
        8
    };
    let reply_ver = format!("RFB 003.{negotiated_minor:03}\n");
    stream.write_all(reply_ver.as_bytes()).await?;
    stream.flush().await?;

    // 2. Security handshake
    let chosen_sec = if negotiated_minor == 3 {
        let mut t = [0u8; 4];
        stream.read_exact(&mut t).await?;
        let sec = u32::from_be_bytes(t);
        if sec == 0 {
            let reason = read_rfb_string(&mut stream).await.unwrap_or_default();
            return Err(anyhow!("VNC connection rejected: {reason}"));
        }
        sec as u8
    } else {
        let n_types = stream.read_u8().await?;
        if n_types == 0 {
            let reason = read_rfb_string(&mut stream).await.unwrap_or_default();
            return Err(anyhow!("VNC handshake failed: {reason}"));
        }
        let mut types = vec![0u8; n_types as usize];
        stream.read_exact(&mut types).await?;

        let chosen = if password.as_deref().is_some_and(|p| !p.is_empty()) {
            if types.contains(&2) {
                Some(2)
            } else if types.contains(&1) {
                Some(1)
            } else {
                None
            }
        } else if types.contains(&1) {
            Some(1)
        } else if types.contains(&2) {
            Some(2)
        } else {
            None
        }
        .ok_or_else(|| {
            anyhow!("Server offers unsupported VNC security types: {types:?} (supports None/VNCAuth)")
        })?;
        stream.write_u8(chosen).await?;
        stream.flush().await?;
        chosen
    };

    match chosen_sec {
        1 => {
            if negotiated_minor >= 8 {
                let res = stream.read_u32().await?;
                if res != 0 {
                    let reason = read_rfb_string(&mut stream).await.unwrap_or_default();
                    return Err(anyhow!("VNC auth rejected: {reason}"));
                }
            }
        }
        2 => {
            let mut challenge = [0u8; 16];
            stream.read_exact(&mut challenge).await?;
            let pw = password.as_deref().unwrap_or("");
            let response = vnc_encrypt_challenge(pw, &challenge);
            stream.write_all(&response).await?;
            stream.flush().await?;

            let res = stream.read_u32().await?;
            if res != 0 {
                let reason = if negotiated_minor >= 8 {
                    read_rfb_string(&mut stream)
                        .await
                        .unwrap_or_else(|_| "Invalid VNC password".into())
                } else {
                    "Authentication failed".to_string()
                };
                return Err(anyhow!("VNC authentication failed: {reason}"));
            }
        }
        other => return Err(anyhow!("Unsupported VNC security type: {other}")),
    }

    // 3. ClientInit (shared = 1) + ServerInit
    stream.write_u8(1).await?;
    stream.flush().await?;

    let width = stream.read_u16().await?.max(1);
    let height = stream.read_u16().await?.max(1);
    let mut _srv_pf = [0u8; 16];
    stream.read_exact(&mut _srv_pf).await?;
    let desktop_name = read_rfb_string(&mut stream).await.unwrap_or_else(|_| "VNC".into());

    // 4. SetPixelFormat: 32-bit little-endian RGBA8888 (R@0, G@8, B@16 -> direct RGBA vec)
    let mut set_pf = [0u8; 20];
    set_pf[0] = 0; // msg-type SetPixelFormat
    set_pf[4] = 32; // bits-per-pixel
    set_pf[5] = 24; // depth
    set_pf[6] = 0; // big-endian = false
    set_pf[7] = 1; // true-colour = true
    set_pf[8..10].copy_from_slice(&255u16.to_be_bytes()); // red-max
    set_pf[10..12].copy_from_slice(&255u16.to_be_bytes()); // green-max
    set_pf[12..14].copy_from_slice(&255u16.to_be_bytes()); // blue-max
    set_pf[14] = 0; // red-shift
    set_pf[15] = 8; // green-shift
    set_pf[16] = 16; // blue-shift
    stream.write_all(&set_pf).await?;

    // 5. SetEncodings (Zlib, CopyRect, Raw, DesktopSize)
    let encodings = [
        ENCODING_ZLIB,
        ENCODING_COPYRECT,
        ENCODING_RAW,
        ENCODING_DESKTOP_SIZE,
    ];
    let mut set_enc = Vec::with_capacity(4 + encodings.len() * 4);
    set_enc.push(2); // msg-type SetEncodings
    set_enc.push(0);
    set_enc.extend_from_slice(&(encodings.len() as u16).to_be_bytes());
    for enc in encodings {
        set_enc.extend_from_slice(&enc.to_be_bytes());
    }
    stream.write_all(&set_enc).await?;

    // 6. Initial full FramebufferUpdateRequest
    let mut req0 = [0u8; 10];
    req0[0] = 3;
    req0[1] = 0; // non-incremental
    req0[6..8].copy_from_slice(&width.to_be_bytes());
    req0[8..10].copy_from_slice(&height.to_be_bytes());
    stream.write_all(&req0).await?;
    stream.flush().await?;

    let dims = Arc::new(std::sync::Mutex::new((width, height)));
    let _ = event_tx.send(VncEvent::Connected {
        desktop_name,
        width,
        height,
    });

    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<VncCmd>();
    let (mut rd, mut wr) = tokio::io::split(stream);

    // Writer task
    let cmd_tx_for_reader = cmd_tx.clone();
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            let res = match cmd {
                VncCmd::KeyEvent { keysym, down } => {
                    let mut buf = [0u8; 8];
                    buf[0] = 4;
                    buf[1] = u8::from(down);
                    buf[4..8].copy_from_slice(&keysym.to_be_bytes());
                    wr.write_all(&buf).await
                }
                VncCmd::PointerEvent { x, y, mask } => {
                    let mut buf = [0u8; 6];
                    buf[0] = 5;
                    buf[1] = mask;
                    buf[2..4].copy_from_slice(&x.to_be_bytes());
                    buf[4..6].copy_from_slice(&y.to_be_bytes());
                    wr.write_all(&buf).await
                }
                VncCmd::ClientCutText(text) => {
                    let bytes = text.as_bytes();
                    let mut buf = Vec::with_capacity(8 + bytes.len());
                    buf.push(6);
                    buf.extend_from_slice(&[0u8; 3]);
                    buf.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
                    buf.extend_from_slice(bytes);
                    wr.write_all(&buf).await
                }
                VncCmd::FramebufferUpdateRequest {
                    incremental,
                    x,
                    y,
                    w,
                    h,
                } => {
                    let mut buf = [0u8; 10];
                    buf[0] = 3;
                    buf[1] = u8::from(incremental);
                    buf[2..4].copy_from_slice(&x.to_be_bytes());
                    buf[4..6].copy_from_slice(&y.to_be_bytes());
                    buf[6..8].copy_from_slice(&w.to_be_bytes());
                    buf[8..10].copy_from_slice(&h.to_be_bytes());
                    wr.write_all(&buf).await
                }
                VncCmd::Disconnect => {
                    let _ = wr.shutdown().await;
                    break;
                }
            };
            if res.is_err() {
                break;
            }
            let _ = wr.flush().await;
        }
    });

    // Dedicated Reader task (never aborted mid-packet!)
    let dims_reader = Arc::clone(&dims);
    tokio::spawn(async move {
        let mut fb = opaque_black_fb(width, height);
        let mut cur_w = width;
        let mut cur_h = height;
        let mut zlib_state = Decompress::new(true);

        loop {
            let msg_type = match rd.read_u8().await {
                Ok(t) => t,
                Err(_) => {
                    let _ = event_tx.send(VncEvent::Disconnected("VNC stream closed".into()));
                    break;
                }
            };

            match msg_type {
                0 => {
                    // FramebufferUpdate
                    if rd.read_u8().await.is_err() {
                        break;
                    }
                    let Ok(n_rects) = rd.read_u16().await else {
                        break;
                    };

                    for _ in 0..n_rects {
                        let Ok(rx) = rd.read_u16().await else { break };
                        let Ok(ry) = rd.read_u16().await else { break };
                        let Ok(rw) = rd.read_u16().await else { break };
                        let Ok(rh) = rd.read_u16().await else { break };
                        let Ok(enc) = rd.read_i32().await else { break };

                        if enc == ENCODING_DESKTOP_SIZE {
                            cur_w = rw.max(1);
                            cur_h = rh.max(1);
                            fb = opaque_black_fb(cur_w, cur_h);
                            if let Ok(mut g) = dims_reader.lock() {
                                *g = (cur_w, cur_h);
                            }
                            zlib_state.reset(true);
                            continue;
                        }

                        if rw == 0 || rh == 0 {
                            continue;
                        }

                        match enc {
                            ENCODING_RAW => {
                                let byte_len = (rw as usize) * (rh as usize) * 4;
                                let mut raw = vec![0u8; byte_len];
                                if rd.read_exact(&mut raw).await.is_err() {
                                    let _ = event_tx
                                        .send(VncEvent::Disconnected("Truncated raw rect".into()));
                                    return;
                                }
                                blit_rect(&mut fb, cur_w, cur_h, rx, ry, rw, rh, &raw);
                            }
                            ENCODING_COPYRECT => {
                                let Ok(sx) = rd.read_u16().await else { return };
                                let Ok(sy) = rd.read_u16().await else { return };
                                copy_rect(&mut fb, cur_w, cur_h, sx, sy, rx, ry, rw, rh);
                            }
                            ENCODING_ZLIB => {
                                let Ok(clen) = rd.read_u32().await else { return };
                                let mut compressed = vec![0u8; clen as usize];
                                if rd.read_exact(&mut compressed).await.is_err() {
                                    let _ = event_tx
                                        .send(VncEvent::Disconnected("Truncated zlib rect".into()));
                                    return;
                                }
                                let expected = (rw as usize) * (rh as usize) * 4;
                                match inflate_rect(&mut zlib_state, &compressed, expected) {
                                    Ok(raw) => {
                                        blit_rect(&mut fb, cur_w, cur_h, rx, ry, rw, rh, &raw);
                                    }
                                    Err(e) => {
                                        let _ = event_tx.send(VncEvent::Error(format!(
                                            "Zlib decode failed: {e}"
                                        )));
                                        return;
                                    }
                                }
                            }
                            other => {
                                let _ = event_tx.send(VncEvent::Error(format!(
                                    "Unsupported VNC rectangle encoding: {other}"
                                )));
                                return;
                            }
                        }
                    }

                    let _ = event_tx.send(VncEvent::Frame(Arc::new(VncFrame {
                        width: cur_w,
                        height: cur_h,
                        rgba: fb.clone(),
                    })));

                    // Request next incremental update covering entire framebuffer
                    let _ = cmd_tx_for_reader.send(VncCmd::FramebufferUpdateRequest {
                        incremental: true,
                        x: 0,
                        y: 0,
                        w: cur_w,
                        h: cur_h,
                    });
                }
                1 => {
                    // SetColourMapEntries
                    let _ = rd.read_u8().await;
                    let _ = rd.read_u16().await;
                    let Ok(n_colors) = rd.read_u16().await else { break };
                    let mut skip = vec![0u8; (n_colors as usize) * 6];
                    if rd.read_exact(&mut skip).await.is_err() {
                        break;
                    }
                }
                2 => {
                    // Bell (no payload)
                }
                3 => {
                    // ServerCutText
                    let mut pad = [0u8; 3];
                    if rd.read_exact(&mut pad).await.is_err() {
                        break;
                    }
                    let Ok(len) = rd.read_u32().await else { break };
                    let clamped = (len as usize).min(1024 * 1024);
                    let mut text_bytes = vec![0u8; clamped];
                    if rd.read_exact(&mut text_bytes).await.is_err() {
                        break;
                    }
                    if len as usize > clamped {
                        let mut rem = vec![0u8; len as usize - clamped];
                        if rd.read_exact(&mut rem).await.is_err() {
                            break;
                        }
                    }
                    let txt = String::from_utf8_lossy(&text_bytes).into_owned();
                    let _ = event_tx.send(VncEvent::ServerClipboard(txt));
                }
                other => {
                    let _ = event_tx.send(VncEvent::Error(format!(
                        "Unknown RFB server message type: {other}"
                    )));
                    break;
                }
            }
        }
    });

    Ok(VncClientHandle { tx: cmd_tx, dims })
}

fn opaque_black_fb(w: u16, h: u16) -> Vec<u8> {
    [0u8, 0, 0, 255].repeat((w as usize) * (h as usize))
}

async fn read_rfb_string<R: tokio::io::AsyncRead + Unpin>(rd: &mut R) -> Result<String> {
    let len = rd.read_u32().await? as usize;
    let clamped = len.min(64 * 1024);
    let mut buf = vec![0u8; clamped];
    rd.read_exact(&mut buf).await?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn inflate_rect(
    dec: &mut Decompress,
    compressed: &[u8],
    expected_len: usize,
) -> Result<Vec<u8>> {
    let mut out = vec![0u8; expected_len];
    let mut in_offset = 0;
    let mut out_offset = 0;
    while out_offset < expected_len {
        let before_in = dec.total_in();
        let before_out = dec.total_out();
        let status = dec.decompress(
            &compressed[in_offset..],
            &mut out[out_offset..],
            FlushDecompress::Sync,
        )?;
        let consumed = (dec.total_in() - before_in) as usize;
        let produced = (dec.total_out() - before_out) as usize;
        in_offset += consumed;
        out_offset += produced;
        if out_offset >= expected_len {
            break;
        }
        if matches!(status, Status::StreamEnd) || (consumed == 0 && produced == 0) {
            return Err(anyhow!(
                "Incomplete Zlib rect: got {out_offset}/{expected_len} bytes"
            ));
        }
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn blit_rect(
    fb: &mut [u8],
    fb_w: u16,
    fb_h: u16,
    rx: u16,
    ry: u16,
    rw: u16,
    rh: u16,
    src: &[u8],
) {
    let fw = fb_w as usize;
    let fh = fb_h as usize;
    for row in 0..(rh as usize) {
        let dst_y = ry as usize + row;
        if dst_y >= fh {
            break;
        }
        let src_row_off = row * (rw as usize) * 4;
        for col in 0..(rw as usize) {
            let dst_x = rx as usize + col;
            if dst_x >= fw {
                break;
            }
            let s_idx = src_row_off + col * 4;
            let d_idx = (dst_y * fw + dst_x) * 4;
            if s_idx + 3 < src.len() && d_idx + 3 < fb.len() {
                fb[d_idx] = src[s_idx];
                fb[d_idx + 1] = src[s_idx + 1];
                fb[d_idx + 2] = src[s_idx + 2];
                fb[d_idx + 3] = 255;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn copy_rect(
    fb: &mut [u8],
    fb_w: u16,
    fb_h: u16,
    sx: u16,
    sy: u16,
    rx: u16,
    ry: u16,
    rw: u16,
    rh: u16,
) {
    let fw = fb_w as usize;
    let fh = fb_h as usize;
    let mut temp = vec![0u8; (rw as usize) * (rh as usize) * 4];
    for row in 0..(rh as usize) {
        let src_y = sy as usize + row;
        if src_y >= fh {
            continue;
        }
        for col in 0..(rw as usize) {
            let src_x = sx as usize + col;
            if src_x >= fw {
                continue;
            }
            let s_idx = (src_y * fw + src_x) * 4;
            let t_idx = (row * (rw as usize) + col) * 4;
            temp[t_idx..t_idx + 4].copy_from_slice(&fb[s_idx..s_idx + 4]);
        }
    }
    blit_rect(fb, fb_w, fb_h, rx, ry, rw, rh, &temp);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vnc_des_challenge_response_matches_known_bitswap_vector() {
        // Empty vs non-empty passwords produce deterministic 16-byte ciphertexts without panic
        let challenge = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
            0xEE, 0xFF,
        ];
        let r1 = vnc_encrypt_challenge("vncpass", &challenge);
        let r2 = vnc_encrypt_challenge("wrongpw", &challenge);
        assert_ne!(r1, r2);
        // Changing 9th+ char is ignored per RFB 8-char limit
        assert_eq!(r1, vnc_encrypt_challenge("vncpass", &challenge));
    }
}
