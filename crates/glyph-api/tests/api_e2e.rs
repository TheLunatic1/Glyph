use glyph_api::{
    ensure_mcp_token, start_local_api, ApiHostKeyPolicy, ApiUiEvent, ApiUiTab,
    LocalVaultApiController,
};
use glyph_core::models::{
    AuthMethod, HostGroup, IdentityInput, ServerConfigInput, SshKeyAlgorithm,
};
use glyph_core::ssh::SshSession;
use glyph_core::vault::VaultManager;
use reqwest::StatusCode;
use russh::keys::ssh_key::{getrandom::SysRng, rand_core::UnwrapErr};
use russh::keys::{Algorithm, PrivateKey};
use russh::server::{self, Auth, ChannelOpenHandle, Config, Handler, Msg, Session};
use russh::{Channel, ChannelId};
use russh_sftp::protocol::{
    Attrs, Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode as SftpStatus,
    Version,
};
use russh_sftp::server::{Handler as SftpHandler, StatusReply};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};

enum OpenFileState {
    File(std::fs::File),
    DirRead {
        entries: Vec<(String, FileAttributes)>,
        done: bool,
    },
}

struct SimpleSftpServer {
    root: PathBuf,
    handles: HashMap<String, OpenFileState>,
    next_id: u64,
}

impl SimpleSftpServer {
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
        for comp in rel.split('/').filter(|s| !s.is_empty() && *s != "." && *s != "..") {
            out.push(comp);
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

    fn ok(id: u32) -> Status {
        Status {
            id,
            status_code: SftpStatus::Ok,
            error_message: "Ok".into(),
            language_tag: "en-US".into(),
        }
    }
}

impl SftpHandler for SimpleSftpServer {
    type Error = StatusReply;

    fn unimplemented(&self) -> Self::Error {
        SftpStatus::OpUnsupported.into()
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
        let rd = fs::read_dir(&target).map_err(|_| SftpStatus::NoSuchFile)?;
        let mut entries = Vec::new();
        for item in rd.flatten() {
            if let Ok(meta) = item.metadata() {
                entries.push((
                    item.file_name().to_string_lossy().to_string(),
                    Self::metadata_attrs(&meta),
                ));
            }
        }
        let handle = format!("d-{}", self.next_id);
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
            return Err(SftpStatus::Failure.into());
        };
        if *done {
            return Err(SftpStatus::Eof.into());
        }
        *done = true;
        let files = entries
            .iter()
            .map(|(n, a)| File::new(n, a.clone()))
            .collect();
        Ok(Name { id, files })
    }

    async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        let meta = fs::metadata(self.resolve(&path)).map_err(|_| SftpStatus::NoSuchFile)?;
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
            return Err(SftpStatus::NoSuchFile.into());
        }
        Ok(Self::ok(id))
    }

    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        _attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        fs::create_dir_all(self.resolve(&path)).map_err(|_| SftpStatus::Failure)?;
        Ok(Self::ok(id))
    }

    async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, Self::Error> {
        fs::remove_dir(self.resolve(&path)).map_err(|_| SftpStatus::Failure)?;
        Ok(Self::ok(id))
    }

    async fn remove(&mut self, id: u32, filename: String) -> Result<Status, Self::Error> {
        fs::remove_file(self.resolve(&filename)).map_err(|_| SftpStatus::NoSuchFile)?;
        Ok(Self::ok(id))
    }

    async fn rename(
        &mut self,
        id: u32,
        oldpath: String,
        newpath: String,
    ) -> Result<Status, Self::Error> {
        fs::rename(self.resolve(&oldpath), self.resolve(&newpath)).map_err(|_| SftpStatus::Failure)?;
        Ok(Self::ok(id))
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
            .truncate(pflags.contains(OpenFlags::TRUNCATE));
        let file = opts.open(&path).map_err(|_| SftpStatus::NoSuchFile)?;
        let handle = format!("h-{}", self.next_id);
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
            return Err(SftpStatus::Failure.into());
        };
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| SftpStatus::Failure)?;
        let mut buf = vec![0u8; len as usize];
        let n = file.read(&mut buf).map_err(|_| SftpStatus::Failure)?;
        if n == 0 {
            return Err(SftpStatus::Eof.into());
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
            return Err(SftpStatus::Failure.into());
        };
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| SftpStatus::Failure)?;
        file.write_all(&data).map_err(|_| SftpStatus::Failure)?;
        Ok(Self::ok(id))
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        self.handles.remove(&handle);
        Ok(Self::ok(id))
    }
}

struct LoopbackSshHandler {
    sftp_root: PathBuf,
    channels: HashMap<ChannelId, Channel<Msg>>,
}

impl Handler for LoopbackSshHandler {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        if user == "tester" && password == "secret-pass" {
            Ok(Auth::Accept)
        } else {
            Ok(Auth::reject())
        }
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

    async fn tcpip_forward(
        &mut self,
        _address: &str,
        port: &mut u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        if *port == 0 {
            *port = 19001;
        }
        Ok(true)
    }

    async fn cancel_tcpip_forward(
        &mut self,
        _address: &str,
        _port: u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        Ok(true)
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
        if cmd.contains("===GLYPH_DELIMITER===") {
            let _ = handle
                .data(
                    channel,
                    "top - 12:00:00 up 3 days, 1 user, load average: 0.25, 0.20, 0.15\n\
Tasks: 100 total, 1 running, 99 sleeping\n\
%Cpu(s): 12.5 us, 2.5 sy, 0.0 ni, 85.0 id\n\
===GLYPH_DELIMITER===\n\
              total        used        free      shared  buff/cache   available\n\
Mem:           8000        2000        4000         100        2000        5900\n\
===GLYPH_DELIMITER===\n\
Filesystem Size Used Avail Use% Mounted on\n\
/dev/sda1 50G 10G 40G 20% /\n\
===GLYPH_DELIMITER===\n\
up 3 days, 2 hours\n\
===GLYPH_DELIMITER===\n\
2\n\
===GLYPH_DELIMITER===\n\
eth0: 1000 2000\n\
===GLYPH_DELIMITER===\n\
cpu  100 0 50 850 0 0 0 0 0 0\n\
===GLYPH_DELIMITER===\n\
\n\
===GLYPH_DELIMITER===\n\
\n\
===GLYPH_DELIMITER===\n\
NO_GPU\n",
                )
                .await;
        } else if cmd.contains("/etc/os-release") {
            let _ = handle.data(channel, "debian\n").await;
        } else if cmd.contains("command -v docker") {
            let _ = handle.data(channel, "docker\n").await;
        } else if cmd.contains("docker ps -a") {
            let _ = handle
                .data(
                    channel,
                    "{\"id\":\"abc\",\"image\":\"redis:7\",\"name\":\"cache\",\"status\":\"Up\",\"state\":\"running\"}\n",
                )
                .await;
        } else if cmd.contains("docker logs") {
            let _ = handle.data(channel, "Ready to accept connections\n").await;
        } else if cmd.contains("docker inspect") {
            let _ = handle
                .data(
                    channel,
                    "[{\"Config\":{\"Cmd\":[\"redis-server\"]},\"HostConfig\":{\"RestartPolicy\":{\"Name\":\"unless-stopped\"}},\"NetworkSettings\":{\"Networks\":{\"bridge\":{}}}}]\n",
                )
                .await;
        } else if cmd.contains("docker stats") {
            let _ = handle.data(channel, "12.4MiB / 8GiB\n").await;
        } else if cmd.contains("docker start")
            || cmd.contains("docker stop")
            || cmd.contains("docker restart")
        {
            let _ = handle.data(channel, "abc\n").await;
        } else if cmd.contains("docker compose") {
            let _ = handle.data(channel, "Container cache Started\n").await;
        } else if let Some(rest) = cmd.strip_prefix("echo ") {
            let _ = handle.data(channel, format!("{}\n", rest.trim())).await;
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
                    russh_sftp::server::run(ch.into_stream(), SimpleSftpServer::new(root)).await;
                });
                return Ok(());
            }
        }
        session.channel_failure(channel)?;
        Ok(())
    }
}

async fn spawn_ssh_server(sftp_root: PathBuf) -> SocketAddr {
    let mut rng = UnwrapErr(SysRng);
    let host_key = PrivateKey::random(&mut rng, Algorithm::Ed25519).expect("host key");
    let config = Arc::new(Config {
        keys: vec![host_key],
        auth_rejection_time: Duration::from_millis(5),
        auth_rejection_time_initial: Some(Duration::from_millis(0)),
        ..Default::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind ssh");
    let addr = listener.local_addr().expect("ssh local_addr");
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let cfg = Arc::clone(&config);
            let root = sftp_root.clone();
            tokio::spawn(async move {
                let handler = LoopbackSshHandler {
                    sftp_root: root,
                    channels: HashMap::new(),
                };
                if let Ok(session) = server::run_stream(cfg, stream, handler).await {
                    let _ = session.await;
                }
            });
        }
    });
    addr
}

async fn spawn_echo_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind echo");
    let addr = listener.local_addr().expect("echo addr");
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 256];
                while let Ok(n) = sock.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    let _ = sock.write_all(&buf[..n]).await;
                }
            });
        }
    });
    addr
}

#[tokio::test]
async fn local_api_all_13_routes_and_auth_e2e() {
    glyph_core::updater::ensure_rustls_ring_provider();
    let uniq = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("glyph-api-e2e-{}-{uniq}", std::process::id()));
    let sftp_root = temp_dir.join("sftp");
    fs::create_dir_all(&sftp_root).expect("mkdir");

    let ssh_addr = spawn_ssh_server(sftp_root.clone()).await;
    let echo_addr = spawn_echo_server().await;

    let mut vault = VaultManager::open_for_test(temp_dir.join("vault"), [42u8; 32]).expect("vault");
    let group_id = vault
        .add_group(HostGroup {
            id: "grp-prod".into(),
            name: "Production".into(),
            parent_id: None,
            identity_id: None,
            proxy_jump_id: None,
            default_port: None,
            tags: vec!["prod".into()],
        })
        .expect("add group");
    let _ssh_key_id = vault
        .generate_ssh_key("Deploy Key", SshKeyAlgorithm::Ed25519, None)
        .expect("gen key");
    let _identity_id = vault
        .add_identity(IdentityInput {
            name: "Ops Identity".into(),
            username: "ops".into(),
            auth_method: AuthMethod::Password,
            password: Some("ultra-secret-identity-pw".into()),
            ..Default::default()
        })
        .expect("add identity");
    let server_id = vault
        .add_server(ServerConfigInput {
            name: "Loopback API Server".into(),
            host: "127.0.0.1".into(),
            username: "tester".into(),
            port: ssh_addr.port(),
            password: Some("secret-pass".into()),
            auth_method: AuthMethod::Password,
            group_id: Some(group_id.clone()),
            tags: vec!["api-test".into()],
            ..Default::default()
        })
        .expect("add server");
    let server_id_2 = vault
        .add_server(ServerConfigInput {
            name: "Loopback Secondary Server".into(),
            host: "127.0.0.1".into(),
            username: "tester".into(),
            port: ssh_addr.port(),
            password: Some("secret-pass".into()),
            auth_method: AuthMethod::Password,
            group_id: Some(group_id.clone()),
            ..Default::default()
        })
        .expect("add server 2");

    let token_path = temp_dir.join("mcp_token");
    let token = ensure_mcp_token(&token_path).expect("token");

    let vault_arc = Arc::new(StdMutex::new(vault));
    let sessions: Arc<Mutex<HashMap<String, SshSession>>> = Arc::new(Mutex::new(HashMap::new()));
    let (ui_tx, mut ui_rx) = mpsc::unbounded_channel::<ApiUiEvent>();

    let controller = Arc::new(LocalVaultApiController::new(
        Arc::clone(&vault_arc),
        Arc::clone(&sessions),
        Some(ui_tx),
        ApiHostKeyPolicy::AutoTrustFirstUse,
    ));

    let api_handle = start_local_api(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        token.clone(),
        controller,
    )
    .await
    .expect("start api");
    let base = format!("http://{}", api_handle.addr());
    let client = reqwest::Client::new();

    // 1. Auth enforcement: missing / wrong token -> 401
    let unauth = client
        .get(format!("{base}/api/servers"))
        .send()
        .await
        .expect("unauth");
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    let wrong = client
        .get(format!("{base}/api/servers"))
        .bearer_auth("wrong-token")
        .send()
        .await
        .expect("wrong auth");
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);

    // 2. GET /api/servers
    let list_res: Vec<Value> = client
        .get(format!("{base}/api/servers"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("servers")
        .json()
        .await
        .expect("servers json");
    assert_eq!(list_res.len(), 2);
    assert_eq!(list_res[0]["id"], server_id);
    assert_eq!(list_res[0]["port"], ssh_addr.port());

    // 3. POST /api/connect
    let conn_res: Value = client
        .post(format!("{base}/api/connect"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id }))
        .send()
        .await
        .expect("connect")
        .json()
        .await
        .expect("connect json");
    assert_eq!(conn_res["success"], true);

    // 4. POST /api/execute
    let exec_res: Value = client
        .post(format!("{base}/api/execute"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "command": "echo api-hello" }))
        .send()
        .await
        .expect("execute")
        .json()
        .await
        .expect("exec json");
    assert_eq!(exec_res["output"].as_str().unwrap_or("").trim(), "api-hello");

    // 5. POST /api/sftp/write & /api/sftp/read
    let write_res: Value = client
        .post(format!("{base}/api/sftp/write"))
        .bearer_auth(&token)
        .json(&json!({
            "serverId": server_id,
            "path": "/hello.txt",
            "content": "pure-rust-sftp"
        }))
        .send()
        .await
        .expect("write")
        .json()
        .await
        .expect("write json");
    assert_eq!(write_res["success"], true);

    let read_res: Value = client
        .post(format!("{base}/api/sftp/read"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "path": "/hello.txt" }))
        .send()
        .await
        .expect("read")
        .json()
        .await
        .expect("read json");
    assert_eq!(read_res["content"], "pure-rust-sftp");

    // 6. POST /api/tunnels/start -> verify traffic -> POST /api/tunnels/stop
    let tun_res: Value = client
        .post(format!("{base}/api/tunnels/start"))
        .bearer_auth(&token)
        .json(&json!({
            "serverId": server_id,
            "localPort": 0,
            "remoteHost": "127.0.0.1",
            "remotePort": echo_addr.port(),
            "protocol": "TCP"
        }))
        .send()
        .await
        .expect("tunnel start")
        .json()
        .await
        .expect("tunnel json");
    let local_port = tun_res["localPort"].as_u64().expect("localPort") as u16;
    assert!(local_port > 0);

    let mut tsock = tokio::net::TcpStream::connect(("127.0.0.1", local_port))
        .await
        .expect("connect tunnel");
    tsock.write_all(b"ping-api-tunnel").await.expect("write");
    let mut rbuf = [0u8; 32];
    let n = tsock.read(&mut rbuf).await.expect("read");
    assert_eq!(&rbuf[..n], b"ping-api-tunnel");
    drop(tsock);

    let stop_res: Value = client
        .post(format!("{base}/api/tunnels/stop"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "localPort": local_port }))
        .send()
        .await
        .expect("tunnel stop")
        .json()
        .await
        .expect("stop json");
    assert_eq!(stop_res["success"], true);

    // 7. Secrets add + list
    let sec_add: Value = client
        .post(format!("{base}/api/secrets/add"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "name": "DB_PASS", "value": "topsecret" }))
        .send()
        .await
        .expect("secret add")
        .json()
        .await
        .expect("sec add json");
    assert_eq!(sec_add["success"], true);

    let sec_list: Vec<Value> = client
        .post(format!("{base}/api/secrets/list"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id }))
        .send()
        .await
        .expect("secret list")
        .json()
        .await
        .expect("sec list json");
    assert_eq!(sec_list.len(), 1);
    assert_eq!(sec_list[0]["name"], "DB_PASS");
    assert!(sec_list[0].get("value").is_none());

    // 8. Containers list
    let cont_res: Value = client
        .post(format!("{base}/api/containers/list"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id }))
        .send()
        .await
        .expect("containers")
        .json()
        .await
        .expect("cont json");
    assert!(cont_res["output"].as_str().unwrap_or("").contains("redis:7"));

    // 9. Commands add + list + remove
    let cmd_add: Value = client
        .post(format!("{base}/api/commands/add"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "name": "Disk Free", "cmd": "df -h" }))
        .send()
        .await
        .expect("cmd add")
        .json()
        .await
        .expect("cmd add json");
    assert_eq!(cmd_add["success"], true);
    let cmd_id = cmd_add["id"].as_u64().expect("cmd id");

    let cmds_resp: Value = client
        .post(format!("{base}/api/commands/list"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id }))
        .send()
        .await
        .expect("cmd list")
        .json()
        .await
        .expect("cmds json");
    let cmds = cmds_resp["commands"].as_array().expect("commands array");
    assert!(cmds.iter().any(|c| c["id"] == cmd_id && c["name"] == "Disk Free"));

    let cmd_rm: Value = client
        .post(format!("{base}/api/commands/remove"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "id": cmd_id }))
        .send()
        .await
        .expect("cmd rm")
        .json()
        .await
        .expect("cmd rm json");
    assert_eq!(cmd_rm["success"], true);

    // 10. New v3 routes: Telemetry
    let stats_res: Value = client
        .post(format!("{base}/api/telemetry"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id }))
        .send()
        .await
        .expect("telemetry")
        .json()
        .await
        .expect("telemetry json");
    assert_eq!(stats_res["uptime"], "up 3 days, 2 hours");
    assert_eq!(stats_res["activeUsers"], "2");

    // 11. New v3 SFTP routes: mkdir, list, rename, chmod, transfer, delete (+ root safety guardrail)
    let mkdir_res: Value = client
        .post(format!("{base}/api/sftp/mkdir"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "path": "/api_dir" }))
        .send()
        .await
        .expect("mkdir")
        .json()
        .await
        .expect("mkdir json");
    assert_eq!(mkdir_res["success"], true);

    let list_dir: Vec<Value> = client
        .post(format!("{base}/api/sftp/list"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "path": "/" }))
        .send()
        .await
        .expect("sftp list")
        .json()
        .await
        .expect("sftp list json");
    assert!(list_dir.iter().any(|e| e["filename"] == "api_dir" && e["isDir"] == true));

    let ren_res: Value = client
        .post(format!("{base}/api/sftp/rename"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "oldPath": "/hello.txt", "newPath": "/api_dir/moved.txt" }))
        .send()
        .await
        .expect("rename")
        .json()
        .await
        .expect("rename json");
    assert_eq!(ren_res["success"], true);

    let chmod_res: Value = client
        .post(format!("{base}/api/sftp/chmod"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "path": "/api_dir/moved.txt", "mode": "755" }))
        .send()
        .await
        .expect("chmod")
        .json()
        .await
        .expect("chmod json");
    assert_eq!(chmod_res["success"], true);
    assert_eq!(chmod_res["mode"], "0755");

    let xfer_res: Value = client
        .post(format!("{base}/api/sftp/transfer"))
        .bearer_auth(&token)
        .json(&json!({
            "sourceServerId": server_id,
            "sourcePath": "/api_dir/moved.txt",
            "destServerId": server_id_2,
            "destPath": "/transferred.txt"
        }))
        .send()
        .await
        .expect("transfer")
        .json()
        .await
        .expect("transfer json");
    assert_eq!(xfer_res["success"], true);
    assert_eq!(xfer_res["bytesTransferred"], "pure-rust-sftp".len() as u64);

    // Security guardrail: refuse deleting "/"
    let unsafe_del = client
        .post(format!("{base}/api/sftp/delete"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "path": "/", "isDir": true }))
        .send()
        .await
        .expect("unsafe delete");
    assert_eq!(unsafe_del.status(), StatusCode::BAD_REQUEST);

    let del_file: Value = client
        .post(format!("{base}/api/sftp/delete"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "path": "/api_dir/moved.txt", "isDir": false }))
        .send()
        .await
        .expect("del file")
        .json()
        .await
        .expect("del json");
    assert_eq!(del_file["success"], true);

    let del_dir: Value = client
        .post(format!("{base}/api/sftp/delete"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "path": "/api_dir", "isDir": true }))
        .send()
        .await
        .expect("del dir")
        .json()
        .await
        .expect("del dir json");
    assert_eq!(del_dir["success"], true);

    // 12. New v3 Tunnels: remote + socks + list + stop by tunnelId
    let rem_tun: Value = client
        .post(format!("{base}/api/tunnels/start_remote"))
        .bearer_auth(&token)
        .json(&json!({
            "serverId": server_id,
            "remoteBindAddr": "127.0.0.1",
            "remotePort": 0,
            "localHost": "127.0.0.1",
            "localPort": echo_addr.port()
        }))
        .send()
        .await
        .expect("remote tun")
        .json()
        .await
        .expect("remote tun json");
    let rem_id = rem_tun["id"].as_str().expect("rem id").to_string();
    assert!(rem_id.starts_with("R:"));

    let socks_tun: Value = client
        .post(format!("{base}/api/tunnels/start_socks"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "localPort": 0 }))
        .send()
        .await
        .expect("socks tun")
        .json()
        .await
        .expect("socks tun json");
    let socks_id = socks_tun["id"].as_str().expect("socks id").to_string();
    assert!(socks_id.starts_with("D:"));

    let tun_list: Vec<Value> = client
        .post(format!("{base}/api/tunnels/list"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id }))
        .send()
        .await
        .expect("tunnels list")
        .json()
        .await
        .expect("tunnels list json");
    assert!(tun_list.iter().any(|t| t["id"] == rem_id));
    assert!(tun_list.iter().any(|t| t["id"] == socks_id));

    for tid in [rem_id, socks_id] {
        let stp: Value = client
            .post(format!("{base}/api/tunnels/stop"))
            .bearer_auth(&token)
            .json(&json!({ "serverId": server_id, "tunnelId": tid }))
            .send()
            .await
            .expect("stop tid")
            .json()
            .await
            .expect("stop tid json");
        assert_eq!(stp["success"], true);
    }

    // 13. New v3 Containers: logs, inspect, action, compose
    let logs_res: Value = client
        .post(format!("{base}/api/containers/logs"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "containerId": "abc", "tail": 50 }))
        .send()
        .await
        .expect("logs")
        .json()
        .await
        .expect("logs json");
    assert!(logs_res["logs"].as_str().unwrap_or("").contains("Ready to accept"));

    let insp_res: Value = client
        .post(format!("{base}/api/containers/inspect"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "containerId": "abc", "isRunning": true }))
        .send()
        .await
        .expect("inspect")
        .json()
        .await
        .expect("inspect json");
    assert_eq!(insp_res["restartPolicy"], "unless-stopped");
    assert_eq!(insp_res["memUsage"], "12.4MiB / 8GiB");

    let act_res: Value = client
        .post(format!("{base}/api/containers/action"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "containerId": "abc", "action": "restart" }))
        .send()
        .await
        .expect("action")
        .json()
        .await
        .expect("action json");
    assert_eq!(act_res["success"], true);

    let cmp_res: Value = client
        .post(format!("{base}/api/containers/compose"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "dir": "/srv/app", "action": "up" }))
        .send()
        .await
        .expect("compose")
        .json()
        .await
        .expect("compose json");
    assert!(cmp_res["output"].as_str().unwrap_or("").contains("Started"));

    // 14. Groups + Group execute + Snippet packages + Read-only security checks + VNC + Disconnect
    let groups_res: Vec<Value> = client
        .get(format!("{base}/api/groups"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("groups")
        .json()
        .await
        .expect("groups json");
    assert!(groups_res.iter().any(|g| g["id"] == group_id));

    let gexec_res: Vec<Value> = client
        .post(format!("{base}/api/groups/execute"))
        .bearer_auth(&token)
        .json(&json!({ "groupId": group_id, "command": "echo group-ok" }))
        .send()
        .await
        .expect("group exec")
        .json()
        .await
        .expect("group exec json");
    assert_eq!(gexec_res.len(), 2);
    assert!(gexec_res.iter().all(|r| r["ok"] == true));

    let pkg_add: Value = client
        .post(format!("{base}/api/snippet_packages/add"))
        .bearer_auth(&token)
        .json(&json!({
            "name": "Ops Pack",
            "description": "Diagnostics",
            "snippets": [{ "name": "Who", "cmd": "whoami" }]
        }))
        .send()
        .await
        .expect("pkg add")
        .json()
        .await
        .expect("pkg add json");
    assert_eq!(pkg_add["name"], "Ops Pack");
    assert_eq!(pkg_add["snippets"].as_array().unwrap().len(), 1);

    let pkgs_res: Vec<Value> = client
        .get(format!("{base}/api/snippet_packages"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("pkgs")
        .json()
        .await
        .expect("pkgs json");
    assert!(pkgs_res.iter().any(|p| p["name"] == "Ops Pack"));

    let keys_raw = client
        .get(format!("{base}/api/ssh_keys"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("ssh keys")
        .text()
        .await
        .expect("ssh keys text");
    assert!(keys_raw.contains("Deploy Key"));
    assert!(!keys_raw.contains("PRIVATE KEY"));

    let ids_raw = client
        .get(format!("{base}/api/identities"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("identities")
        .text()
        .await
        .expect("identities text");
    assert!(ids_raw.contains("Ops Identity"));
    assert!(!ids_raw.contains("ultra-secret-identity-pw"));

    let kh_res: Vec<Value> = client
        .get(format!("{base}/api/known_hosts"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("known hosts")
        .json()
        .await
        .expect("kh json");
    assert!(!kh_res.is_empty());

    let vnc_res: Value = client
        .post(format!("{base}/api/vnc/open"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id, "host": "127.0.0.1", "port": 5900, "autoConnect": false }))
        .send()
        .await
        .expect("vnc")
        .json()
        .await
        .expect("vnc json");
    assert_eq!(vnc_res["success"], true);

    let disc_res: Value = client
        .post(format!("{base}/api/disconnect"))
        .bearer_auth(&token)
        .json(&json!({ "serverId": server_id }))
        .send()
        .await
        .expect("disconnect")
        .json()
        .await
        .expect("disc json");
    assert_eq!(disc_res["success"], true);

    let mut saw_terminal_tab = false;
    let mut saw_cmd_started = false;
    let mut saw_cmd_output = false;
    let mut saw_sftp_read = false;
    let mut saw_sftp_write = false;
    let mut saw_telemetry = false;
    let mut saw_sftp_dir = false;
    let mut saw_sftp_refresh = false;
    let mut saw_container_detail = false;
    let mut saw_compose = false;
    let mut saw_vnc = false;
    let mut saw_disconnect = false;
    let mut saw_vault_changed = false;
    while let Ok(ev) = ui_rx.try_recv() {
        match ev {
            ApiUiEvent::SwitchTab {
                tab: ApiUiTab::Terminal,
                ..
            } => saw_terminal_tab = true,
            ApiUiEvent::AgentCommandStarted { .. } => saw_cmd_started = true,
            ApiUiEvent::AgentCommandOutput { .. } => saw_cmd_output = true,
            ApiUiEvent::AgentSftpFile { action, .. } if action == "read" => saw_sftp_read = true,
            ApiUiEvent::AgentSftpFile { action, .. } if action == "write" => saw_sftp_write = true,
            ApiUiEvent::AgentTelemetryUpdated { .. } => saw_telemetry = true,
            ApiUiEvent::AgentSftpDir { .. } => saw_sftp_dir = true,
            ApiUiEvent::AgentSftpRefresh { .. } => saw_sftp_refresh = true,
            ApiUiEvent::AgentContainerDetail { .. } => saw_container_detail = true,
            ApiUiEvent::AgentComposeResult { .. } => saw_compose = true,
            ApiUiEvent::AgentOpenVnc { .. } => saw_vnc = true,
            ApiUiEvent::SessionDisconnected { .. } => saw_disconnect = true,
            ApiUiEvent::VaultChanged => saw_vault_changed = true,
            _ => {}
        }
    }
    assert!(saw_terminal_tab);
    assert!(saw_cmd_started);
    assert!(saw_cmd_output);
    assert!(saw_sftp_read);
    assert!(saw_sftp_write);
    assert!(saw_telemetry);
    assert!(saw_sftp_dir);
    assert!(saw_sftp_refresh);
    assert!(saw_container_detail);
    assert!(saw_compose);
    assert!(saw_vnc);
    assert!(saw_disconnect);
    assert!(saw_vault_changed);

    api_handle.shutdown().await;
    let _ = fs::remove_dir_all(&temp_dir);
}
