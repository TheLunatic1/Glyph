use flate2::{Compress, Compression, FlushCompress};
use glyph_core::models::{AuthMethod, ConnectionConfig};
use glyph_core::ssh::{HostKeyPrompt, SessionEvent, SshSession};
use glyph_vnc::{connect_vnc_stream, vnc_encrypt_challenge, VncEvent};
use russh::keys::{Algorithm, HashAlg, PrivateKey};
use russh::server::{self, Auth, ChannelOpenHandle, Config, Handler, Msg, Session};
use russh::{Channel, ChannelId};
use ssh_key::{getrandom::SysRng, rand_core::UnwrapErr};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};

// ── Mock RFB 3.8 Server ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum MockInputEvent {
    Key { down: bool, keysym: u32 },
    Pointer { mask: u8, x: u16, y: u16 },
    CutText(String),
}

async fn spawn_mock_rfb_server(
    expected_password: &'static str,
    input_log: Arc<Mutex<Vec<MockInputEvent>>>,
) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let join = tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            let log_ref = Arc::clone(&input_log);
            tokio::spawn(async move {
                // 1. Version
                if sock.write_all(b"RFB 003.008\n").await.is_err() {
                    return;
                }
                let mut cli_ver = [0u8; 12];
                if sock.read_exact(&mut cli_ver).await.is_err() {
                    return;
                }

                // 2. Security types: offer [2] (VNC Auth)
                let _ = sock.write_all(&[1u8, 2u8]).await;
                let Ok(chosen) = sock.read_u8().await else { return };
                if chosen != 2 {
                    return;
                }

                let challenge: [u8; 16] = [
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
                ];
                let _ = sock.write_all(&challenge).await;
                let mut resp = [0u8; 16];
                if sock.read_exact(&mut resp).await.is_err() {
                    return;
                }

                let expected = vnc_encrypt_challenge(expected_password, &challenge);
                if resp != expected {
                    let _ = sock.write_all(&1u32.to_be_bytes()).await;
                    let reason = b"Invalid password";
                    let _ = sock.write_all(&(reason.len() as u32).to_be_bytes()).await;
                    let _ = sock.write_all(reason).await;
                    return;
                }
                let _ = sock.write_all(&0u32.to_be_bytes()).await;

                // 3. ClientInit + ServerInit (8x8)
                let Ok(_shared) = sock.read_u8().await else { return };
                let mut srv_init = Vec::new();
                srv_init.extend_from_slice(&8u16.to_be_bytes());
                srv_init.extend_from_slice(&8u16.to_be_bytes());
                srv_init.extend_from_slice(&[32, 24, 0, 1, 0, 255, 0, 255, 0, 255, 0, 8, 16, 0, 0, 0]);
                let name = b"Glyph-Mock-VNC";
                srv_init.extend_from_slice(&(name.len() as u32).to_be_bytes());
                srv_init.extend_from_slice(name);
                let _ = sock.write_all(&srv_init).await;

                let mut update_step = 0u8;
                let mut z_comp = Compress::new(Compression::default(), true);

                while let Ok(mtype) = sock.read_u8().await {
                    match mtype {
                        0 => {
                            let mut rest = [0u8; 19];
                            if sock.read_exact(&mut rest).await.is_err() {
                                break;
                            }
                        }
                        2 => {
                            let _ = sock.read_u8().await;
                            let Ok(cnt) = sock.read_u16().await else { break };
                            let mut encs = vec![0u8; (cnt as usize) * 4];
                            if sock.read_exact(&mut encs).await.is_err() {
                                break;
                            }
                        }
                        3 => {
                            let mut req = [0u8; 9];
                            if sock.read_exact(&mut req).await.is_err() {
                                break;
                            }
                            if update_step == 0 {
                                // Frame 1: Raw red 4x4 at (0,0) + CopyRect to (4,0)
                                let mut upd = Vec::new();
                                upd.push(0); // FramebufferUpdate
                                upd.push(0);
                                upd.extend_from_slice(&2u16.to_be_bytes()); // 2 rects
                                // Rect 1: Raw 4x4 @ (0,0) -> wire alpha=0 intentionally
                                upd.extend_from_slice(&0u16.to_be_bytes());
                                upd.extend_from_slice(&0u16.to_be_bytes());
                                upd.extend_from_slice(&4u16.to_be_bytes());
                                upd.extend_from_slice(&4u16.to_be_bytes());
                                upd.extend_from_slice(&0i32.to_be_bytes());
                                for _ in 0..16 {
                                    upd.extend_from_slice(&[255, 0, 0, 0]);
                                }
                                // Rect 2: CopyRect 4x4 from (0,0) to (4,0)
                                upd.extend_from_slice(&4u16.to_be_bytes());
                                upd.extend_from_slice(&0u16.to_be_bytes());
                                upd.extend_from_slice(&4u16.to_be_bytes());
                                upd.extend_from_slice(&4u16.to_be_bytes());
                                upd.extend_from_slice(&1i32.to_be_bytes());
                                upd.extend_from_slice(&0u16.to_be_bytes());
                                upd.extend_from_slice(&0u16.to_be_bytes());

                                let _ = sock.write_all(&upd).await;
                                update_step = 1;
                            } else if update_step == 1 {
                                // ServerCutText + Frame 2: Zlib green 4x4 at (0,4) + DesktopSize 16x12
                                let clip = b"remote-clip";
                                let mut cut = vec![3, 0, 0, 0];
                                cut.extend_from_slice(&(clip.len() as u32).to_be_bytes());
                                cut.extend_from_slice(clip);
                                let _ = sock.write_all(&cut).await;

                                let mut green_raw = Vec::with_capacity(64);
                                for _ in 0..16 {
                                    green_raw.extend_from_slice(&[0, 255, 0, 0]);
                                }
                                let mut z_out = vec![0u8; 256];
                                let b_out = z_comp.total_out();
                                let _ = z_comp.compress(&green_raw, &mut z_out, FlushCompress::Sync);
                                let produced = (z_comp.total_out() - b_out) as usize;
                                z_out.truncate(produced);

                                let mut upd = Vec::new();
                                upd.push(0);
                                upd.push(0);
                                upd.extend_from_slice(&1u16.to_be_bytes());
                                upd.extend_from_slice(&0u16.to_be_bytes());
                                upd.extend_from_slice(&4u16.to_be_bytes());
                                upd.extend_from_slice(&4u16.to_be_bytes());
                                upd.extend_from_slice(&4u16.to_be_bytes());
                                upd.extend_from_slice(&6i32.to_be_bytes()); // Zlib
                                upd.extend_from_slice(&(z_out.len() as u32).to_be_bytes());
                                upd.extend_from_slice(&z_out);
                                let _ = sock.write_all(&upd).await;
                                update_step = 2;
                            } else if update_step == 2 {
                                // Frame 3: DesktopSize resize -> 16x12
                                let mut upd = Vec::new();
                                upd.push(0);
                                upd.push(0);
                                upd.extend_from_slice(&1u16.to_be_bytes());
                                upd.extend_from_slice(&0u16.to_be_bytes());
                                upd.extend_from_slice(&0u16.to_be_bytes());
                                upd.extend_from_slice(&16u16.to_be_bytes());
                                upd.extend_from_slice(&12u16.to_be_bytes());
                                upd.extend_from_slice(&(-223i32).to_be_bytes());
                                let _ = sock.write_all(&upd).await;
                                update_step = 3;
                            }
                        }
                        4 => {
                            let Ok(down) = sock.read_u8().await else { break };
                            let mut pad = [0u8; 2];
                            if sock.read_exact(&mut pad).await.is_err() {
                                break;
                            }
                            let Ok(keysym) = sock.read_u32().await else { break };
                            log_ref.lock().await.push(MockInputEvent::Key {
                                down: down != 0,
                                keysym,
                            });
                        }
                        5 => {
                            let Ok(mask) = sock.read_u8().await else { break };
                            let Ok(x) = sock.read_u16().await else { break };
                            let Ok(y) = sock.read_u16().await else { break };
                            log_ref
                                .lock()
                                .await
                                .push(MockInputEvent::Pointer { mask, x, y });
                        }
                        6 => {
                            let mut pad = [0u8; 3];
                            if sock.read_exact(&mut pad).await.is_err() {
                                break;
                            }
                            let Ok(len) = sock.read_u32().await else { break };
                            let mut buf = vec![0u8; len as usize];
                            if sock.read_exact(&mut buf).await.is_err() {
                                break;
                            }
                            log_ref.lock().await.push(MockInputEvent::CutText(
                                String::from_utf8_lossy(&buf).into_owned(),
                            ));
                        }
                        _ => break,
                    }
                }
            });
        }
    });

    (port, join)
}

// ── Minimal SSH Server with direct-tcpip tunneling ────────────────────────────

struct TunnelOnlySshHandler;

impl Handler for TunnelOnlySshHandler {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        if user == "vncuser" && password == "sshpass" {
            Ok(Auth::Accept)
        } else {
            Ok(Auth::reject())
        }
    }

    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    async fn exec_request(
        &mut self,
        channel: ChannelId,
        _data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        let handle = session.handle();
        let _ = handle.data(channel, "ubuntu\n").await;
        let _ = handle.exit_status_request(channel, 0).await;
        let _ = handle.eof(channel).await;
        let _ = handle.close(channel).await;
        Ok(())
    }

    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<Msg>,
        host_to_connect: &str,
        port_to_connect: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Ok(mut target) =
            tokio::net::TcpStream::connect((host_to_connect, port_to_connect as u16)).await
        {
            reply.accept().await;
            tokio::spawn(async move {
                let mut stream = channel.into_stream();
                let _ = tokio::io::copy_bidirectional(&mut stream, &mut target).await;
            });
        } else {
            reply
                .reject(russh::ChannelOpenFailure::ConnectFailed)
                .await;
        }
        Ok(())
    }
}

#[tokio::test]
async fn vnc_over_ssh_tunnel_auth_render_zlib_resize_and_input_e2e() {
    let input_log = Arc::new(Mutex::new(Vec::new()));
    let (vnc_port, vnc_task) = spawn_mock_rfb_server("vncpass", Arc::clone(&input_log)).await;

    // 1. Direct wrong-password test
    {
        let sock = tokio::net::TcpStream::connect(("127.0.0.1", vnc_port))
            .await
            .unwrap();
        let (ev_tx, _ev_rx) = mpsc::unbounded_channel();
        let err = connect_vnc_stream(sock, Some("wrongpw".into()), ev_tx)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Invalid password"));
    }

    // 2. Spin up SSH server and tunnel VNC over `open_direct_tcpip`
    let host_key = PrivateKey::random(&mut UnwrapErr(SysRng), Algorithm::Ed25519).unwrap();
    let fp = host_key
        .public_key()
        .fingerprint(HashAlg::Sha256)
        .to_string();
    let cfg = Arc::new(Config {
        keys: vec![host_key],
        auth_rejection_time: Duration::from_millis(10),
        auth_rejection_time_initial: Some(Duration::from_millis(0)),
        ..Default::default()
    });

    let ssh_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ssh_port = ssh_listener.local_addr().unwrap().port();
    let ssh_task = tokio::spawn(async move {
        while let Ok((stream, _)) = ssh_listener.accept().await {
            let cfg_c = Arc::clone(&cfg);
            tokio::spawn(async move {
                if let Ok(sess) = server::run_stream(cfg_c, stream, TunnelOnlySshHandler).await {
                    let _ = sess.await;
                }
            });
        }
    });

    let mut known_hosts = HashMap::new();
    known_hosts.insert(format!("127.0.0.1:{ssh_port}"), fp);

    let (ev_tx, mut ssh_ev_rx) = mpsc::unbounded_channel::<SessionEvent>();
    let (hk_tx, _hk_rx) = mpsc::unbounded_channel::<HostKeyPrompt>();
    tokio::spawn(async move { while ssh_ev_rx.recv().await.is_some() {} });

    let ssh_session = SshSession::connect(
        ConnectionConfig {
            host: "127.0.0.1".into(),
            port: ssh_port,
            username: "vncuser".into(),
            password: Some("sshpass".into()),
            auth_method: AuthMethod::Password,
            ..Default::default()
        },
        known_hosts,
        ev_tx,
        hk_tx,
    )
    .await
    .unwrap()
    .session;

    let tunnel_stream = ssh_session
        .open_direct_tcpip("127.0.0.1", vnc_port)
        .await
        .unwrap();

    let (vnc_tx, mut vnc_rx) = mpsc::unbounded_channel::<VncEvent>();
    let vnc_handle = connect_vnc_stream(tunnel_stream, Some("vncpass".into()), vnc_tx)
        .await
        .unwrap();

    vnc_handle.send_pointer(2, 3, 1);
    vnc_handle.send_key(b'g' as u32, true);
    vnc_handle.send_key(b'g' as u32, false);
    vnc_handle.send_clipboard("hello-from-glyph".into());

    let mut connected_name = None;
    let mut saw_red_copy = false;
    let mut saw_green_zlib = false;
    let mut saw_resize = false;
    let mut saw_clip = false;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    while tokio::time::Instant::now() < deadline
        && !(connected_name.is_some()
            && saw_red_copy
            && saw_green_zlib
            && saw_resize
            && saw_clip)
    {
        if let Ok(Some(ev)) = tokio::time::timeout(Duration::from_millis(500), vnc_rx.recv()).await
        {
            match ev {
                VncEvent::Connected {
                    desktop_name,
                    width,
                    height,
                } => {
                    assert_eq!((width, height), (8, 8));
                    connected_name = Some(desktop_name);
                }
                VncEvent::ServerClipboard(s) => {
                    assert_eq!(s, "remote-clip");
                    saw_clip = true;
                }
                VncEvent::Frame(frame) => {
                    if frame.width == 8 && frame.height == 8 {
                        // Pixel (0,0) Raw Red with alpha=255
                        if frame.rgba[0..4] == [255, 0, 0, 255]
                            // Pixel (4,0) CopyRect Red with alpha=255
                            && frame.rgba[4 * 4..4 * 4 + 4] == [255, 0, 0, 255]
                        {
                            saw_red_copy = true;
                        }
                        // Pixel (0,4) Zlib Green with alpha=255
                        let row4_off = 4 * 8 * 4;
                        if frame.rgba[row4_off..row4_off + 4] == [0, 255, 0, 255] {
                            saw_green_zlib = true;
                        }
                    } else if frame.width == 16 && frame.height == 12 {
                        assert_eq!(frame.rgba.len(), 16 * 12 * 4);
                        assert_eq!(frame.rgba[3], 255);
                        saw_resize = true;
                    }
                }
                VncEvent::Error(e) => panic!("VNC error: {e}"),
                VncEvent::Disconnected(_) => {}
            }
        }
    }

    assert_eq!(connected_name.as_deref(), Some("Glyph-Mock-VNC"));
    assert!(saw_red_copy, "Missing Raw + CopyRect frame");
    assert!(saw_green_zlib, "Missing Zlib green rect");
    assert!(saw_resize, "Missing DesktopSize 16x12 resize");
    assert!(saw_clip, "Missing ServerCutText");

    tokio::time::sleep(Duration::from_millis(100)).await;
    let recorded = input_log.lock().await;
    assert!(recorded.contains(&MockInputEvent::Pointer {
        mask: 1,
        x: 2,
        y: 3
    }));
    assert!(recorded.contains(&MockInputEvent::Key {
        down: true,
        keysym: b'g' as u32
    }));
    assert!(recorded.contains(&MockInputEvent::CutText("hello-from-glyph".into())));

    vnc_handle.disconnect();
    ssh_session.disconnect().await;
    ssh_task.abort();
    vnc_task.abort();
}
