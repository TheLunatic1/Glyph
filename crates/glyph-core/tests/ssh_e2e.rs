use glyph_core::containers::ContainerAction;
use glyph_core::models::{AuthMethod, ConnectionConfig, HostKeyStatus};
use glyph_core::sftp::SftpProgress;
use glyph_core::ssh::{HostKeyPrompt, SessionEvent, ShellEvent, SshSession};
use glyph_core::tunnels::TunnelDirection;
use glyph_core::vault::VaultManager;
use russh::keys::ssh_key::{getrandom::SysRng, rand_core::UnwrapErr};
use russh::keys::{Algorithm, HashAlg, PrivateKey, PublicKey};
use russh::server::{self, Auth, ChannelOpenHandle, Config, Handler, Msg, Session};
use russh::{Channel, ChannelId, Pty};
use russh_sftp::protocol::{
    Attrs, Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode, Version,
};
use russh_sftp::server::{Handler as SftpHandler, StatusReply};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

// ── Real Loopback SFTP Server Handler backed by temp filesystem ───────────────

enum OpenFileState {
    File(std::fs::File),
    DirRead {
        entries: Vec<(String, FileAttributes)>,
        done: bool,
    },
}

struct TempSftpServer {
    root: PathBuf,
    handles: HashMap<String, OpenFileState>,
    next_id: u64,
}

impl TempSftpServer {
    fn new(root: PathBuf) -> Self {
        Self {
            root,
            handles: HashMap::new(),
            next_id: 1,
        }
    }

    fn resolve(&self, remote: &str) -> PathBuf {
        let rel = remote.trim_start_matches('/');
        if rel.is_empty() || rel == "." {
            return self.root.clone();
        }
        let mut out = self.root.clone();
        for comp in rel.split('/').filter(|s| !s.is_empty() && *s != ".") {
            if comp != ".." {
                out.push(comp);
            }
        }
        out
    }

    fn metadata_attrs(meta: &fs::Metadata) -> FileAttributes {
        FileAttributes {
            size: Some(meta.len()),
            permissions: Some(if meta.is_dir() { 0o040755 } else { 0o100644 }),
            mtime: Some(1_700_000_000),
            atime: Some(1_700_000_000),
            ..Default::default()
        }
    }

    fn ok_status(id: u32) -> Status {
        Status {
            id,
            status_code: StatusCode::Ok,
            error_message: "Ok".into(),
            language_tag: "en-US".into(),
        }
    }
}

impl SftpHandler for TempSftpServer {
    type Error = StatusReply;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported.into()
    }

    async fn init(
        &mut self,
        _version: u32,
        _extensions: HashMap<String, String>,
    ) -> Result<Version, Self::Error> {
        Ok(Version::new())
    }

    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        let clean = if path.trim().is_empty() || path == "." {
            "/".to_string()
        } else if path.starts_with('/') {
            path
        } else {
            format!("/{path}")
        };
        Ok(Name {
            id,
            files: vec![File::dummy(clean)],
        })
    }

    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, Self::Error> {
        let target = self.resolve(&path);
        let rd = fs::read_dir(&target).map_err(|_| StatusCode::NoSuchFile)?;
        let mut entries = Vec::new();
        for item in rd.flatten() {
            if let Ok(meta) = item.metadata() {
                entries.push((
                    item.file_name().to_string_lossy().to_string(),
                    Self::metadata_attrs(&meta),
                ));
            }
        }
        let handle = format!("dir-{}", self.next_id);
        self.next_id += 1;
        self.handles.insert(
            handle.clone(),
            OpenFileState::DirRead {
                entries,
                done: false,
            },
        );
        Ok(Handle { id, handle })
    }

    async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, Self::Error> {
        let Some(OpenFileState::DirRead { entries, done }) = self.handles.get_mut(&handle) else {
            return Err(StatusCode::Failure.into());
        };
        if *done {
            return Err(StatusCode::Eof.into());
        }
        *done = true;
        let files = entries
            .iter()
            .map(|(n, a)| File::new(n, a.clone()))
            .collect();
        Ok(Name { id, files })
    }

    async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        let meta = fs::metadata(self.resolve(&path)).map_err(|_| StatusCode::NoSuchFile)?;
        Ok(Attrs {
            id,
            attrs: Self::metadata_attrs(&meta),
        })
    }

    async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        self.stat(id, path).await
    }

    async fn setstat(
        &mut self,
        id: u32,
        path: String,
        _attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        if !self.resolve(&path).exists() {
            return Err(StatusCode::NoSuchFile.into());
        }
        Ok(Self::ok_status(id))
    }

    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        _attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        fs::create_dir_all(self.resolve(&path)).map_err(|_| StatusCode::Failure)?;
        Ok(Self::ok_status(id))
    }

    async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, Self::Error> {
        fs::remove_dir(self.resolve(&path)).map_err(|_| StatusCode::Failure)?;
        Ok(Self::ok_status(id))
    }

    async fn remove(&mut self, id: u32, filename: String) -> Result<Status, Self::Error> {
        fs::remove_file(self.resolve(&filename)).map_err(|_| StatusCode::NoSuchFile)?;
        Ok(Self::ok_status(id))
    }

    async fn rename(
        &mut self,
        id: u32,
        oldpath: String,
        newpath: String,
    ) -> Result<Status, Self::Error> {
        fs::rename(self.resolve(&oldpath), self.resolve(&newpath)).map_err(|_| StatusCode::Failure)?;
        Ok(Self::ok_status(id))
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        pflags: OpenFlags,
        _attrs: FileAttributes,
    ) -> Result<Handle, Self::Error> {
        let path = self.resolve(&filename);
        if pflags.contains(OpenFlags::CREATE) {
            if let Some(p) = path.parent() {
                let _ = fs::create_dir_all(p);
            }
        }
        let mut opts = OpenOptions::new();
        opts.read(pflags.contains(OpenFlags::READ) || !pflags.contains(OpenFlags::WRITE))
            .write(pflags.contains(OpenFlags::WRITE) || pflags.contains(OpenFlags::CREATE))
            .create(pflags.contains(OpenFlags::CREATE))
            .truncate(pflags.contains(OpenFlags::TRUNCATE))
            .append(pflags.contains(OpenFlags::APPEND));

        let file = opts.open(&path).map_err(|_| StatusCode::NoSuchFile)?;
        let handle = format!("file-{}", self.next_id);
        self.next_id += 1;
        self.handles.insert(handle.clone(), OpenFileState::File(file));
        Ok(Handle { id, handle })
    }

    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, Self::Error> {
        let Some(OpenFileState::File(file)) = self.handles.get_mut(&handle) else {
            return Err(StatusCode::Failure.into());
        };
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| StatusCode::Failure)?;
        let mut buf = vec![0u8; len as usize];
        let n = file.read(&mut buf).map_err(|_| StatusCode::Failure)?;
        if n == 0 {
            return Err(StatusCode::Eof.into());
        }
        buf.truncate(n);
        Ok(Data { id, data: buf })
    }

    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Self::Error> {
        let Some(OpenFileState::File(file)) = self.handles.get_mut(&handle) else {
            return Err(StatusCode::Failure.into());
        };
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| StatusCode::Failure)?;
        file.write_all(&data).map_err(|_| StatusCode::Failure)?;
        Ok(Self::ok_status(id))
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        self.handles.remove(&handle);
        Ok(Self::ok_status(id))
    }
}

// ── Loopback SSH Server Handler ───────────────────────────────────────────────

struct MockSshServerHandler {
    sftp_root: PathBuf,
    authorized_keys: Arc<Mutex<Vec<String>>>,
    channels: HashMap<ChannelId, Channel<Msg>>,
    shell_channels: std::collections::HashSet<ChannelId>,
}

impl Handler for MockSshServerHandler {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        if user == "tester" && password == "secret-pass" {
            Ok(Auth::Accept)
        } else {
            Ok(Auth::reject())
        }
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        key: russh::server::VerifiedPublicKey<'_>,
    ) -> Result<Auth, Self::Error> {
        if user != "tester" {
            return Ok(Auth::reject());
        }
        let offered_fp = key.public_key.fingerprint(HashAlg::Sha256).to_string();
        let keys = self.authorized_keys.lock().await;
        for line in keys.iter() {
            if let Ok(pk) = PublicKey::from_openssh(line) {
                if pk.fingerprint(HashAlg::Sha256).to_string() == offered_fp {
                    return Ok(Auth::Accept);
                }
            }
        }
        Ok(Auth::reject())
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.insert(channel.id(), channel);
        reply.accept().await;
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
        let addr = format!("{host_to_connect}:{port_to_connect}");
        match tokio::net::TcpStream::connect(&addr).await {
            Ok(mut target) => {
                reply.accept().await;
                tokio::spawn(async move {
                    let mut stream = channel.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut stream, &mut target).await;
                });
            }
            Err(_) => {
                reply
                    .reject(russh::ChannelOpenFailure::ConnectFailed)
                    .await;
            }
        }
        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _term: &str,
        _col_width: u32,
        _row_height: u32,
        _pix_width: u32,
        _pix_height: u32,
        _modes: &[(Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        Ok(())
    }

    async fn window_change_request(
        &mut self,
        channel: ChannelId,
        _col_width: u32,
        _row_height: u32,
        _pix_width: u32,
        _pix_height: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.shell_channels.insert(channel);
        session.channel_success(channel)?;
        let handle = session.handle();
        let _ = handle.data(channel, "PTY_READY\r\n").await;
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if self.shell_channels.contains(&channel) {
            let handle = session.handle();
            let _ = handle.data(channel, data.to_vec()).await;
        }
        Ok(())
    }

    async fn exec_request(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        let cmd = String::from_utf8_lossy(data).to_string();
        let handle = session.handle();

        if cmd.contains("/etc/os-release") {
            let _ = handle.data(channel, "ubuntu\n").await;
        } else if cmd.contains("authorized_keys") {
            if let Some(idx) = cmd.find("ssh-") {
                let tail = &cmd[idx..];
                if let Some(end) = tail.find('\'') {
                    let pubkey = tail[..end].replace("'\\''", "'");
                    self.authorized_keys.lock().await.push(pubkey);
                }
            }
        } else if cmd.contains("command -v docker") {
            let _ = handle.data(channel, "docker\n").await;
        } else if cmd.contains("docker ps -a") {
            let _ = handle
                .data(
                    channel,
                    "{\"id\":\"c1\",\"image\":\"nginx:alpine\",\"name\":\"web\",\"status\":\"Up 2h\",\"state\":\"running\"}\n",
                )
                .await;
        } else if cmd.contains("docker restart c1") {
            let _ = handle.data(channel, "c1\n").await;
        } else if cmd.starts_with("echo ") {
            let rest = cmd.trim_start_matches("echo ").trim();
            let _ = handle.data(channel, format!("{rest}\n")).await;
        } else if cmd.starts_with("rm -rf ") {
            // Simple recursive cleanup under temp root
            if let Some(arg) = cmd.strip_prefix("rm -rf ") {
                let unquoted = arg.trim().trim_matches('\'');
                let rel = unquoted.trim_start_matches('/');
                let target = self.sftp_root.join(rel);
                let _ = fs::remove_dir_all(target);
            }
        }

        let _ = handle.exit_status_request(channel, 0).await;
        let _ = handle.eof(channel).await;
        let _ = handle.close(channel).await;
        Ok(())
    }

    async fn subsystem_request(
        &mut self,
        channel: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if name == "sftp" {
            if let Some(ch) = self.channels.remove(&channel) {
                session.channel_success(channel)?;
                let root = self.sftp_root.clone();
                tokio::spawn(async move {
                    russh_sftp::server::run(ch.into_stream(), TempSftpServer::new(root)).await;
                });
                return Ok(());
            }
        }
        session.channel_failure(channel)?;
        Ok(())
    }
}

async fn spawn_mock_ssh_server(
    sftp_root: PathBuf,
    authorized_keys: Arc<Mutex<Vec<String>>>,
) -> (SocketAddr, String, tokio::task::JoinHandle<()>) {
    let host_key = PrivateKey::random(&mut UnwrapErr(SysRng), Algorithm::Ed25519).unwrap();
    let expected_fp = host_key
        .public_key()
        .fingerprint(HashAlg::Sha256)
        .to_string();

    let cfg = Arc::new(Config {
        keys: vec![host_key],
        auth_rejection_time: Duration::from_millis(10),
        auth_rejection_time_initial: Some(Duration::from_millis(0)),
        ..Default::default()
    });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let join = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let cfg_c = Arc::clone(&cfg);
            let root_c = sftp_root.clone();
            let ak_c = Arc::clone(&authorized_keys);
            tokio::spawn(async move {
                let handler = MockSshServerHandler {
                    sftp_root: root_c,
                    authorized_keys: ak_c,
                    channels: HashMap::new(),
                    shell_channels: std::collections::HashSet::new(),
                };
                if let Ok(sess) = server::run_stream(cfg_c, stream, handler).await {
                    let _ = sess.await;
                }
            });
        }
    });

    (addr, expected_fp, join)
}

#[tokio::test]
async fn full_loopback_ssh_tofu_keygen_pty_sftp_tunnel_and_containers_e2e() {
    let base_dir = std::env::temp_dir().join(format!("glyph_ssh_e2e_{}", Uuid::new_v4()));
    let sftp_root = base_dir.join("remote_fs");
    let local_stage = base_dir.join("local");
    fs::create_dir_all(&sftp_root).unwrap();
    fs::create_dir_all(&local_stage).unwrap();

    let authorized_keys = Arc::new(Mutex::new(Vec::new()));
    let (ssh_addr, host_fp, server_task) =
        spawn_mock_ssh_server(sftp_root.clone(), Arc::clone(&authorized_keys)).await;

    // Also start a real TCP echo backend on 127.0.0.1:0 for Local Tunnel (-L) test
    let echo_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let echo_port = echo_listener.local_addr().unwrap().port();
    let echo_task = tokio::spawn(async move {
        while let Ok((mut sock, _)) = echo_listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                while let Ok(n) = sock.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    let _ = sock.write_all(b"ECHO:").await;
                    let _ = sock.write_all(&buf[..n]).await;
                }
            });
        }
    });

    let mut vault = VaultManager::open_for_test(base_dir.join("vault"), [42u8; 32]).unwrap();
    let key_summary = vault
        .generate_ssh_key(
            "e2e-ed25519",
            glyph_core::models::SshKeyAlgorithm::Ed25519,
            None,
        )
        .unwrap();

    // 1. Verify TOFU unknown rejection first
    {
        let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<SessionEvent>();
        let (hk_tx, mut hk_rx) = mpsc::unbounded_channel::<HostKeyPrompt>();
        let host_fp_expected = host_fp.clone();
        tokio::spawn(async move {
            while let Some(p) = hk_rx.recv().await {
                assert_eq!(p.fingerprint_sha256, host_fp_expected);
                assert!(p.mismatch_expected.is_none());
                let _ = p.responder.send(false);
            }
        });
        tokio::spawn(async move { while ev_rx.recv().await.is_some() {} });

        let cfg = ConnectionConfig {
            host: "127.0.0.1".into(),
            port: ssh_addr.port(),
            username: "tester".into(),
            password: Some("secret-pass".into()),
            private_key_path: None,
            private_key_pem: None,
            passphrase: None,
            auth_method: AuthMethod::Password,
            zerotier: String::new(),
            proxy_jump: None,
        };
        assert!(
            SshSession::connect(cfg, vault.known_hosts_map(), ev_tx, hk_tx)
                .await
                .is_err(),
            "Rejecting TOFU host key must abort connection"
        );
    }

    // 2. Connect accepting TOFU + password, deploy Vault SSH public key
    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<SessionEvent>();
    let (hk_tx, mut hk_rx) = mpsc::unbounded_channel::<HostKeyPrompt>();
    tokio::spawn(async move {
        while let Some(p) = hk_rx.recv().await {
            let _ = p.responder.send(true);
        }
    });

    let cfg = ConnectionConfig {
        host: "127.0.0.1".into(),
        port: ssh_addr.port(),
        username: "tester".into(),
        password: Some("secret-pass".into()),
        private_key_path: None,
        private_key_pem: None,
        passphrase: None,
        auth_method: AuthMethod::Password,
        zerotier: String::new(),
        proxy_jump: None,
    };
    let outcome = SshSession::connect(cfg, vault.known_hosts_map(), ev_tx, hk_tx)
        .await
        .unwrap();
    assert_eq!(outcome.os.as_deref(), Some("ubuntu"));
    assert_eq!(outcome.newly_trusted_hosts.len(), 1);
    for rec in &outcome.newly_trusted_hosts {
        vault
            .trust_host_key(&rec.host, rec.port, &rec.algorithm, &rec.fingerprint_sha256)
            .unwrap();
    }
    assert_eq!(
        vault.verify_host_key("127.0.0.1", ssh_addr.port(), &host_fp),
        HostKeyStatus::Trusted
    );
    // Mismatch test
    assert!(matches!(
        vault.verify_host_key("127.0.0.1", ssh_addr.port(), "SHA256:fake"),
        HostKeyStatus::Mismatch { .. }
    ));

    outcome
        .session
        .deploy_public_key(&key_summary.public_key_openssh)
        .await
        .unwrap();
    outcome.session.disconnect().await;
    while ev_rx.recv().await.is_some() {}

    // 3. Reconnect purely using the deployed Vault SSH Key + trusted known_hosts (zero prompt)
    let key_pem = vault.get_decrypted_ssh_key(&key_summary.id).unwrap();
    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<SessionEvent>();
    let (hk_tx, mut hk_rx) = mpsc::unbounded_channel::<HostKeyPrompt>();
    tokio::spawn(async move {
        if hk_rx.recv().await.is_some() {
            panic!("Trusted known_host should NOT prompt again");
        }
    });

    let key_cfg = ConnectionConfig {
        host: "127.0.0.1".into(),
        port: ssh_addr.port(),
        username: "tester".into(),
        password: None,
        private_key_path: None,
        private_key_pem: Some(key_pem),
        passphrase: None,
        auth_method: AuthMethod::VaultKey,
        zerotier: String::new(),
        proxy_jump: None,
    };
    let session = SshSession::connect(key_cfg, vault.known_hosts_map(), ev_tx.clone(), hk_tx)
        .await
        .unwrap()
        .session;

    // 4. Test exec + PTY shell write/read/resize
    let echo_out = session.exec("echo hello_glyph_v3").await.unwrap();
    assert_eq!(echo_out.trim(), "hello_glyph_v3");

    let shell = session
        .open_shell("tab-e2e".into(), 100, 30, ev_tx.clone())
        .await
        .unwrap();
    shell.resize(120, 40);
    shell.write(b"ping_pty\r".to_vec());

    let mut shell_output = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline && !shell_output.contains("ping_pty") {
        if let Ok(Some(SessionEvent::Shell(ShellEvent::Data { bytes, .. }))) =
            tokio::time::timeout(Duration::from_millis(400), ev_rx.recv()).await
        {
            shell_output.push_str(&String::from_utf8_lossy(&bytes));
        }
    }
    assert!(
        shell_output.contains("PTY_READY") && shell_output.contains("ping_pty"),
        "Unexpected PTY output: {shell_output:?}"
    );
    session.close_shell("tab-e2e").await;

    // 5. Test SFTP mkdir, write_text, read_text, readdir, rename, upload (with progress), download, delete
    session.sftp_mkdir("/proj").await.unwrap();
    session
        .sftp_write_text("/proj/hello.txt", "pure rust sftp ok\n")
        .await
        .unwrap();
    let read_back = session.sftp_read_text("/proj/hello.txt").await.unwrap();
    assert_eq!(read_back, "pure rust sftp ok\n");

    let listing = session.sftp_readdir("/proj").await.unwrap();
    assert_eq!(listing.len(), 1);
    assert_eq!(listing[0].filename, "hello.txt");

    session
        .sftp_rename("/proj/hello.txt", "/proj/renamed.txt")
        .await
        .unwrap();
    assert_eq!(
        session.sftp_read_text("/proj/renamed.txt").await.unwrap(),
        "pure rust sftp ok\n"
    );

    let local_up = local_stage.join("upload.bin");
    let payload: Vec<u8> = (0..80_000).map(|i| (i % 251) as u8).collect();
    fs::write(&local_up, &payload).unwrap();
    let (ptx, mut prx) = mpsc::unbounded_channel::<SftpProgress>();
    let uploaded = session
        .sftp_upload_file(&local_up, "/proj/upload.bin", Some(ptx))
        .await
        .unwrap();
    assert_eq!(uploaded, payload.len() as u64);
    let mut saw_progress = false;
    while let Ok(p) = prx.try_recv() {
        if p.transferred == payload.len() as u64 {
            saw_progress = true;
        }
    }
    assert!(saw_progress);

    let local_down = local_stage.join("downloaded.bin");
    let downloaded = session
        .sftp_download_file("/proj/upload.bin", &local_down, None)
        .await
        .unwrap();
    assert_eq!(downloaded, payload.len() as u64);
    assert_eq!(fs::read(&local_down).unwrap(), payload);

    session.sftp_delete("/proj/renamed.txt", false).await.unwrap();
    session.sftp_delete("/proj", true).await.unwrap();

    // 6. Test Local SSH Tunnel (-L) forwarding live TCP stream
    let tunnel = session
        .start_local_tunnel(0, "127.0.0.1".into(), echo_port, "tcp".into())
        .await
        .unwrap();
    assert_eq!(tunnel.direction, TunnelDirection::Local);
    let mut cli = tokio::net::TcpStream::connect(("127.0.0.1", tunnel.local_port))
        .await
        .unwrap();
    cli.write_all(b"glyph-tunnel-test").await.unwrap();
    let mut rbuf = [0u8; 64];
    let n = cli.read(&mut rbuf).await.unwrap();
    assert_eq!(&rbuf[..n], b"ECHO:glyph-tunnel-test");
    drop(cli);

    assert_eq!(session.list_tunnels().await.len(), 1);
    session.stop_tunnel(tunnel.local_port).await.unwrap();
    assert!(session.list_tunnels().await.is_empty());

    // 7. Test Containers command routing over SSH
    let (_raw, containers) = session.list_containers().await.unwrap();
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].name, "web");
    session
        .container_action("c1", ContainerAction::Restart)
        .await
        .unwrap();

    session.disconnect().await;
    server_task.abort();
    echo_task.abort();
    let _ = fs::remove_dir_all(base_dir);
}
