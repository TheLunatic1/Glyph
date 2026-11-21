use glyph_api::{
    ensure_mcp_token, start_local_api, ApiHostKeyPolicy, ApiUiEvent, ApiUiTab,
    LocalVaultApiController,
};
use glyph_core::models::{AuthMethod, ServerConfigInput};
use glyph_core::ssh::SshSession;
use glyph_core::vault::VaultManager;
use reqwest::StatusCode;
use russh::keys::ssh_key::{getrandom::SysRng, rand_core::UnwrapErr};
use russh::keys::{Algorithm, PrivateKey};
use russh::server::{self, Auth, ChannelOpenHandle, Config, Handler, Msg, Session};
use russh::{Channel, ChannelId};
use russh_sftp::protocol::{FileAttributes, Handle, OpenFlags, Status, StatusCode as SftpStatus, Version, Data};
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

struct SimpleSftpServer {
    root: PathBuf,
    handles: HashMap<String, std::fs::File>,
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
        let mut out = self.root.clone();
        for comp in rel.split('/').filter(|s| !s.is_empty() && *s != "." && *s != "..") {
            out.push(comp);
        }
        out
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
        self.handles.insert(handle.clone(), file);
        Ok(Handle { id, handle })
    }

    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, Self::Error> {
        let Some(file) = self.handles.get_mut(&handle) else {
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
        let Some(file) = self.handles.get_mut(&handle) else {
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
    let server_id = vault
        .add_server(ServerConfigInput {
            name: "Loopback API Server".into(),
            host: "127.0.0.1".into(),
            username: "tester".into(),
            port: ssh_addr.port(),
            password: Some("secret-pass".into()),
            auth_method: AuthMethod::Password,
            ..Default::default()
        })
        .expect("add server");

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
    assert_eq!(list_res.len(), 1);
    assert_eq!(list_res[0]["id"], server_id);

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

    let mut saw_terminal_tab = false;
    let mut saw_cmd_started = false;
    let mut saw_cmd_output = false;
    let mut saw_sftp_read = false;
    let mut saw_sftp_write = false;
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
            _ => {}
        }
    }
    assert!(saw_terminal_tab);
    assert!(saw_cmd_started);
    assert!(saw_cmd_output);
    assert!(saw_sftp_read);
    assert!(saw_sftp_write);

    api_handle.shutdown().await;
    let _ = fs::remove_dir_all(&temp_dir);
}
