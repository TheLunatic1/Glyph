use glyph_api::{
    ensure_mcp_token, start_local_api, ApiHostKeyPolicy, LocalVaultApiController,
};
use glyph_core::models::{
    AuthMethod, HostGroup, IdentityInput, ServerConfigInput, SshKeyAlgorithm,
};
use glyph_core::ssh::SshSession;
use glyph_core::vault::VaultManager;
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
use std::collections::{BTreeSet, HashMap};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::sync::Mutex;

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
        let handle = format!("m-{}", self.next_id);
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

struct LoopbackMcpSshHandler {
    sftp_root: PathBuf,
    channels: HashMap<ChannelId, Channel<Msg>>,
}

impl Handler for LoopbackMcpSshHandler {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        if user == "mcpuser" && password == "mcppass" {
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
            *port = 19002;
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
                    "top - 12:00:00 up 5 days, 1 user, load average: 0.10, 0.15, 0.12\n\
Tasks: 80 total, 1 running, 79 sleeping\n\
%Cpu(s): 8.0 us, 2.0 sy, 0.0 ni, 90.0 id\n\
===GLYPH_DELIMITER===\n\
              total        used        free      shared  buff/cache   available\n\
Mem:           4000        1000        2000          50        1000        2950\n\
===GLYPH_DELIMITER===\n\
Filesystem Size Used Avail Use% Mounted on\n\
/dev/sda1 40G 8G 32G 20% /\n\
===GLYPH_DELIMITER===\n\
up 5 days, 1 hour\n\
===GLYPH_DELIMITER===\n\
1\n\
===GLYPH_DELIMITER===\n\
eth0: 500 900\n\
===GLYPH_DELIMITER===\n\
cpu  80 0 20 900 0 0 0 0 0 0\n\
===GLYPH_DELIMITER===\n\
\n\
===GLYPH_DELIMITER===\n\
\n\
===GLYPH_DELIMITER===\n\
NO_GPU\n",
                )
                .await;
        } else if cmd.contains("/etc/os-release") {
            let _ = handle.data(channel, "alpine\n").await;
        } else if cmd.contains("command -v docker") {
            let _ = handle.data(channel, "docker\n").await;
        } else if cmd.contains("docker ps -a") {
            let _ = handle
                .data(
                    channel,
                    "{\"id\":\"c99\",\"image\":\"postgres:16\",\"name\":\"db\",\"status\":\"Up 1h\",\"state\":\"running\"}\n",
                )
                .await;
        } else if cmd.contains("docker logs") {
            let _ = handle.data(channel, "database system is ready\n").await;
        } else if cmd.contains("docker inspect") {
            let _ = handle
                .data(
                    channel,
                    "[{\"Config\":{\"Cmd\":[\"postgres\"]},\"HostConfig\":{\"RestartPolicy\":{\"Name\":\"always\"}},\"NetworkSettings\":{\"Networks\":{\"pgnet\":{}}}}]\n",
                )
                .await;
        } else if cmd.contains("docker stats") {
            let _ = handle.data(channel, "64MiB / 4GiB\n").await;
        } else if cmd.contains("docker start")
            || cmd.contains("docker stop")
            || cmd.contains("docker restart")
        {
            let _ = handle.data(channel, "c99\n").await;
        } else if cmd.contains("docker compose") {
            let _ = handle.data(channel, "Container db Stopped\n").await;
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
    let host_key = PrivateKey::random(&mut UnwrapErr(SysRng), Algorithm::Ed25519).expect("key");
    let cfg = Arc::new(Config {
        keys: vec![host_key],
        auth_rejection_time: Duration::from_millis(5),
        auth_rejection_time_initial: Some(Duration::from_millis(0)),
        ..Default::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let c = Arc::clone(&cfg);
            let r = sftp_root.clone();
            tokio::spawn(async move {
                let handler = LoopbackMcpSshHandler {
                    sftp_root: r,
                    channels: HashMap::new(),
                };
                if let Ok(sess) = server::run_stream(c, stream, handler).await {
                    let _ = sess.await;
                }
            });
        }
    });
    addr
}

async fn spawn_echo_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
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

async fn rpc_call(
    stdin: &mut tokio::process::ChildStdin,
    stdout_lines: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    msg: Value,
) -> Value {
    let mut raw = serde_json::to_string(&msg).expect("serialize");
    raw.push('\n');
    stdin.write_all(raw.as_bytes()).await.expect("write stdin");
    stdin.flush().await.expect("flush stdin");

    let line = tokio::time::timeout(Duration::from_secs(10), stdout_lines.next_line())
        .await
        .expect("rpc timeout")
        .expect("read stdout")
        .expect("unexpected EOF from glyph-mcp");
    serde_json::from_str(&line).expect("parse jsonrpc response")
}

async fn call_tool(
    stdin: &mut tokio::process::ChildStdin,
    stdout_lines: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    id: u64,
    tool: &str,
    arguments: Value,
) -> (bool, Value) {
    let resp = rpc_call(
        stdin,
        stdout_lines,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": tool,
                "arguments": arguments
            }
        }),
    )
    .await;
    assert_eq!(resp["id"], id);
    let is_error = resp["result"]["isError"].as_bool().unwrap_or(true);
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let parsed = serde_json::from_str(&text).unwrap_or(Value::String(text));
    (is_error, parsed)
}

#[tokio::test]
async fn glyph_mcp_stdio_all_13_tools_e2e() {
    let uniq = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("glyph-mcp-e2e-{}-{uniq}", std::process::id()));
    let sftp_root = temp_dir.join("sftp");
    fs::create_dir_all(&sftp_root).expect("mkdir");

    let ssh_addr = spawn_ssh_server(sftp_root.clone()).await;
    let echo_addr = spawn_echo_server().await;

    let mut vault = VaultManager::open_for_test(temp_dir.join("vault"), [77u8; 32]).expect("vault");
    let group_id = vault
        .add_group(HostGroup {
            id: "grp-mcp".into(),
            name: "MCP Fleet".into(),
            parent_id: None,
            identity_id: None,
            proxy_jump_id: None,
            default_port: None,
            tags: vec!["fleet".into()],
        })
        .expect("add group");
    let _ssh_key_id = vault
        .generate_ssh_key("MCP Key", SshKeyAlgorithm::Ed25519, None)
        .expect("gen key");
    let _identity_id = vault
        .add_identity(IdentityInput {
            name: "MCP Identity".into(),
            username: "mcpadmin".into(),
            auth_method: AuthMethod::Password,
            password: Some("never-leak-over-mcp".into()),
            ..Default::default()
        })
        .expect("add identity");
    let server_id = vault
        .add_server(ServerConfigInput {
            name: "MCP Test Box".into(),
            host: "127.0.0.1".into(),
            username: "mcpuser".into(),
            port: ssh_addr.port(),
            password: Some("mcppass".into()),
            auth_method: AuthMethod::Password,
            group_id: Some(group_id.clone()),
            tags: vec!["primary".into()],
            ..Default::default()
        })
        .expect("add server");
    let server_id_2 = vault
        .add_server(ServerConfigInput {
            name: "MCP Test Box 2".into(),
            host: "127.0.0.1".into(),
            username: "mcpuser".into(),
            port: ssh_addr.port(),
            password: Some("mcppass".into()),
            auth_method: AuthMethod::Password,
            group_id: Some(group_id.clone()),
            ..Default::default()
        })
        .expect("add server 2");

    let token_path = temp_dir.join("mcp_token");
    let token = ensure_mcp_token(&token_path).expect("token");

    let vault_arc = Arc::new(StdMutex::new(vault));
    let sessions: Arc<Mutex<HashMap<String, SshSession>>> = Arc::new(Mutex::new(HashMap::new()));
    let controller = Arc::new(LocalVaultApiController::new(
        Arc::clone(&vault_arc),
        Arc::clone(&sessions),
        None,
        ApiHostKeyPolicy::AutoTrustFirstUse,
    ));

    let api_handle = start_local_api(SocketAddr::from(([127, 0, 0, 1], 0)), token, controller)
        .await
        .expect("start api");
    let api_url = format!("http://{}", api_handle.addr());

    let mut child = Command::new(env!("CARGO_BIN_EXE_glyph-mcp"))
        .env("GLYPH_API_URL", &api_url)
        .env("GLYPH_MCP_TOKEN_FILE", &token_path)
        .env("GLYPH_NO_AUTOLAUNCH", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn glyph-mcp");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut lines = BufReader::new(stdout).lines();

    // 1. initialize
    let init = rpc_call(
        &mut stdin,
        &mut lines,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "mcp-e2e", "version": "1.0" }
            }
        }),
    )
    .await;
    assert_eq!(init["result"]["serverInfo"]["name"], "glyph-ssh-manager");

    // 2. notifications/initialized produces no response; follow with ping
    stdin
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .await
        .expect("notify");
    let ping = rpc_call(
        &mut stdin,
        &mut lines,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" }),
    )
    .await;
    assert_eq!(ping["id"], 2);
    assert!(ping["result"].is_object());

    // 3. tools/list asserts all 36 exact tool names
    let list = rpc_call(
        &mut stdin,
        &mut lines,
        json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list" }),
    )
    .await;
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 36);
    let names: BTreeSet<&str> = tools
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    for expected in [
        "glyph_list_servers",
        "glyph_connect_server",
        "glyph_disconnect_server",
        "glyph_execute_command",
        "glyph_get_server_stats",
        "glyph_read_file",
        "glyph_write_file",
        "glyph_sftp_list",
        "glyph_sftp_mkdir",
        "glyph_sftp_rename",
        "glyph_sftp_chmod",
        "glyph_sftp_delete",
        "glyph_sftp_transfer",
        "glyph_start_tunnel",
        "glyph_start_remote_tunnel",
        "glyph_start_socks_tunnel",
        "glyph_list_tunnels",
        "glyph_stop_tunnel",
        "glyph_list_secrets",
        "glyph_add_secret",
        "glyph_list_containers",
        "glyph_container_logs",
        "glyph_container_inspect",
        "glyph_container_action",
        "glyph_compose_action",
        "glyph_list_commands",
        "glyph_add_command",
        "glyph_remove_command",
        "glyph_list_groups",
        "glyph_run_group_command",
        "glyph_list_snippet_packages",
        "glyph_add_snippet_package",
        "glyph_list_ssh_keys",
        "glyph_list_identities",
        "glyph_list_known_hosts",
        "glyph_open_vnc",
    ] {
        assert!(names.contains(expected), "missing tool {expected}");
    }

    // 4. All 13 tools/call roundtrips
    let (err, servers_val) = call_tool(&mut stdin, &mut lines, 10, "glyph_list_servers", json!({})).await;
    assert!(!err);
    assert_eq!(servers_val[0]["id"], server_id);

    let (err, conn_val) = call_tool(
        &mut stdin,
        &mut lines,
        11,
        "glyph_connect_server",
        json!({ "serverId": server_id }),
    )
    .await;
    assert!(!err);
    assert_eq!(conn_val["success"], true);

    let (err, exec_val) = call_tool(
        &mut stdin,
        &mut lines,
        12,
        "glyph_execute_command",
        json!({ "serverId": server_id, "command": "echo mcp-ok" }),
    )
    .await;
    assert!(!err);
    assert_eq!(exec_val["output"].as_str().unwrap_or("").trim(), "mcp-ok");

    let (err, write_val) = call_tool(
        &mut stdin,
        &mut lines,
        13,
        "glyph_write_file",
        json!({ "serverId": server_id, "path": "/mcp.txt", "content": "mcp-sftp-data" }),
    )
    .await;
    assert!(!err);
    assert_eq!(write_val["success"], true);

    let (err, read_val) = call_tool(
        &mut stdin,
        &mut lines,
        14,
        "glyph_read_file",
        json!({ "serverId": server_id, "path": "/mcp.txt" }),
    )
    .await;
    assert!(!err);
    assert_eq!(read_val["content"], "mcp-sftp-data");

    let (err, tun_start) = call_tool(
        &mut stdin,
        &mut lines,
        15,
        "glyph_start_tunnel",
        json!({
            "serverId": server_id,
            "localPort": 0,
            "remoteHost": "127.0.0.1",
            "remotePort": echo_addr.port(),
            "protocol": "TCP"
        }),
    )
    .await;
    assert!(!err);
    let local_port = tun_start["localPort"].as_u64().expect("localPort") as u16;
    assert!(local_port > 0);

    let mut tsock = tokio::net::TcpStream::connect(("127.0.0.1", local_port))
        .await
        .expect("connect forwarded port");
    tsock.write_all(b"mcp-tunnel").await.expect("write");
    let mut rbuf = [0u8; 32];
    let n = tsock.read(&mut rbuf).await.expect("read");
    assert_eq!(&rbuf[..n], b"mcp-tunnel");
    drop(tsock);

    let (err, tun_stop) = call_tool(
        &mut stdin,
        &mut lines,
        16,
        "glyph_stop_tunnel",
        json!({ "serverId": server_id, "localPort": local_port }),
    )
    .await;
    assert!(!err);
    assert_eq!(tun_stop["success"], true);
    assert!(
        tokio::net::TcpStream::connect(("127.0.0.1", local_port))
            .await
            .is_err()
    );

    let (err, sec_add) = call_tool(
        &mut stdin,
        &mut lines,
        17,
        "glyph_add_secret",
        json!({ "serverId": server_id, "name": "JWT_KEY", "value": "s3cr3t" }),
    )
    .await;
    assert!(!err);
    assert_eq!(sec_add["success"], true);

    let (err, sec_list) = call_tool(
        &mut stdin,
        &mut lines,
        18,
        "glyph_list_secrets",
        json!({ "serverId": server_id }),
    )
    .await;
    assert!(!err);
    assert_eq!(sec_list[0]["name"], "JWT_KEY");

    let (err, cont_list) = call_tool(
        &mut stdin,
        &mut lines,
        19,
        "glyph_list_containers",
        json!({ "serverId": server_id }),
    )
    .await;
    assert!(!err);
    assert!(cont_list["output"]
        .as_str()
        .unwrap_or("")
        .contains("postgres:16"));

    let (err, cmd_add) = call_tool(
        &mut stdin,
        &mut lines,
        20,
        "glyph_add_command",
        json!({ "serverId": server_id, "name": "Uptime", "cmd": "uptime" }),
    )
    .await;
    assert!(!err);
    let cmd_id = cmd_add["id"].as_u64().expect("cmd id");

    let (err, cmd_list) = call_tool(
        &mut stdin,
        &mut lines,
        21,
        "glyph_list_commands",
        json!({ "serverId": server_id }),
    )
    .await;
    assert!(!err);
    assert!(cmd_list
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["id"] == cmd_id && c["name"] == "Uptime"));

    let (err, cmd_rm) = call_tool(
        &mut stdin,
        &mut lines,
        22,
        "glyph_remove_command",
        json!({ "serverId": server_id, "id": cmd_id }),
    )
    .await;
    assert!(!err);
    assert_eq!(cmd_rm["success"], true);

    // 5. All 23 new v3 MCP tools/call roundtrips + security checks
    let (err, stats_val) = call_tool(
        &mut stdin,
        &mut lines,
        30,
        "glyph_get_server_stats",
        json!({ "serverId": server_id }),
    )
    .await;
    assert!(!err);
    assert_eq!(stats_val["uptime"], "up 5 days, 1 hour");

    let (err, mk_val) = call_tool(
        &mut stdin,
        &mut lines,
        31,
        "glyph_sftp_mkdir",
        json!({ "serverId": server_id, "path": "/mcp_dir" }),
    )
    .await;
    assert!(!err);
    assert_eq!(mk_val["success"], true);

    let (err, ls_val) = call_tool(
        &mut stdin,
        &mut lines,
        32,
        "glyph_sftp_list",
        json!({ "serverId": server_id, "path": "/" }),
    )
    .await;
    assert!(!err);
    assert!(ls_val
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["filename"] == "mcp_dir" && e["isDir"] == true));

    let (err, mv_val) = call_tool(
        &mut stdin,
        &mut lines,
        33,
        "glyph_sftp_rename",
        json!({ "serverId": server_id, "oldPath": "/mcp.txt", "newPath": "/mcp_dir/moved.txt" }),
    )
    .await;
    assert!(!err);
    assert_eq!(mv_val["success"], true);

    let (err, ch_val) = call_tool(
        &mut stdin,
        &mut lines,
        34,
        "glyph_sftp_chmod",
        json!({ "serverId": server_id, "path": "/mcp_dir/moved.txt", "mode": "640" }),
    )
    .await;
    assert!(!err);
    assert_eq!(ch_val["mode"], "0640");

    let (err, xfer_val) = call_tool(
        &mut stdin,
        &mut lines,
        35,
        "glyph_sftp_transfer",
        json!({
            "sourceServerId": server_id,
            "sourcePath": "/mcp_dir/moved.txt",
            "destServerId": server_id_2,
            "destPath": "/mcp_transferred.txt"
        }),
    )
    .await;
    assert!(!err);
    assert_eq!(xfer_val["success"], true);
    assert_eq!(xfer_val["bytesTransferred"], "mcp-sftp-data".len() as u64);

    // Unsafe root delete rejected
    let (err, _) = call_tool(
        &mut stdin,
        &mut lines,
        36,
        "glyph_sftp_delete",
        json!({ "serverId": server_id, "path": "/", "isDir": true }),
    )
    .await;
    assert!(err);

    let (err, _) = call_tool(
        &mut stdin,
        &mut lines,
        37,
        "glyph_sftp_delete",
        json!({ "serverId": server_id, "path": "/mcp_dir/moved.txt", "isDir": false }),
    )
    .await;
    assert!(!err);

    let (err, rem_tun) = call_tool(
        &mut stdin,
        &mut lines,
        38,
        "glyph_start_remote_tunnel",
        json!({
            "serverId": server_id,
            "remotePort": 0,
            "localPort": echo_addr.port()
        }),
    )
    .await;
    assert!(!err);
    let rem_tid = rem_tun["id"].as_str().expect("rem_tid").to_string();

    let (err, socks_tun) = call_tool(
        &mut stdin,
        &mut lines,
        39,
        "glyph_start_socks_tunnel",
        json!({ "serverId": server_id, "localPort": 0 }),
    )
    .await;
    assert!(!err);
    let socks_tid = socks_tun["id"].as_str().expect("socks_tid").to_string();

    let (err, tuns) = call_tool(
        &mut stdin,
        &mut lines,
        40,
        "glyph_list_tunnels",
        json!({ "serverId": server_id }),
    )
    .await;
    assert!(!err);
    assert!(tuns.as_array().unwrap().iter().any(|t| t["id"] == rem_tid));
    assert!(tuns.as_array().unwrap().iter().any(|t| t["id"] == socks_tid));

    for (idx, tid) in [rem_tid, socks_tid].into_iter().enumerate() {
        let (err, _) = call_tool(
            &mut stdin,
            &mut lines,
            41 + idx as u64,
            "glyph_stop_tunnel",
            json!({ "serverId": server_id, "tunnelId": tid }),
        )
        .await;
        assert!(!err);
    }

    let (err, logs_val) = call_tool(
        &mut stdin,
        &mut lines,
        43,
        "glyph_container_logs",
        json!({ "serverId": server_id, "containerId": "c99", "tail": 50 }),
    )
    .await;
    assert!(!err);
    assert!(logs_val["logs"].as_str().unwrap_or("").contains("ready"));

    let (err, insp_val) = call_tool(
        &mut stdin,
        &mut lines,
        44,
        "glyph_container_inspect",
        json!({ "serverId": server_id, "containerId": "c99", "isRunning": true }),
    )
    .await;
    assert!(!err);
    assert_eq!(insp_val["restartPolicy"], "always");

    let (err, act_val) = call_tool(
        &mut stdin,
        &mut lines,
        45,
        "glyph_container_action",
        json!({ "serverId": server_id, "containerId": "c99", "action": "restart" }),
    )
    .await;
    assert!(!err);
    assert_eq!(act_val["success"], true);

    let (err, cmp_val) = call_tool(
        &mut stdin,
        &mut lines,
        46,
        "glyph_compose_action",
        json!({ "serverId": server_id, "dir": "/srv/pg", "action": "down" }),
    )
    .await;
    assert!(!err);
    assert!(cmp_val["output"].as_str().unwrap_or("").contains("Stopped"));

    let (err, grps) = call_tool(&mut stdin, &mut lines, 47, "glyph_list_groups", json!({})).await;
    assert!(!err);
    assert!(grps.as_array().unwrap().iter().any(|g| g["id"] == group_id));

    let (err, gexec) = call_tool(
        &mut stdin,
        &mut lines,
        48,
        "glyph_run_group_command",
        json!({ "groupId": group_id, "command": "echo fleet-ok" }),
    )
    .await;
    assert!(!err);
    assert_eq!(gexec.as_array().unwrap().len(), 2);

    let (err, pkg_val) = call_tool(
        &mut stdin,
        &mut lines,
        49,
        "glyph_add_snippet_package",
        json!({
            "name": "MCP Pack",
            "description": "Fleet check",
            "snippets": [{ "name": "Host", "cmd": "hostname" }]
        }),
    )
    .await;
    assert!(!err);
    assert_eq!(pkg_val["name"], "MCP Pack");

    let (err, pkgs) = call_tool(
        &mut stdin,
        &mut lines,
        50,
        "glyph_list_snippet_packages",
        json!({}),
    )
    .await;
    assert!(!err);
    assert!(pkgs.as_array().unwrap().iter().any(|p| p["name"] == "MCP Pack"));

    let (err, keys) = call_tool(&mut stdin, &mut lines, 51, "glyph_list_ssh_keys", json!({})).await;
    assert!(!err);
    assert!(keys.as_array().unwrap().iter().any(|k| k["name"] == "MCP Key"));
    assert!(!keys.to_string().contains("PRIVATE KEY"));

    let (err, ids) = call_tool(&mut stdin, &mut lines, 52, "glyph_list_identities", json!({})).await;
    assert!(!err);
    assert!(ids.as_array().unwrap().iter().any(|i| i["name"] == "MCP Identity"));
    assert!(!ids.to_string().contains("never-leak-over-mcp"));

    let (err, kh) = call_tool(&mut stdin, &mut lines, 53, "glyph_list_known_hosts", json!({})).await;
    assert!(!err);
    assert!(!kh.as_array().unwrap().is_empty());

    let (err, vnc) = call_tool(
        &mut stdin,
        &mut lines,
        54,
        "glyph_open_vnc",
        json!({ "serverId": server_id, "host": "127.0.0.1", "port": 5900, "autoConnect": false }),
    )
    .await;
    assert!(!err);
    assert_eq!(vnc["success"], true);

    let (err, disc) = call_tool(
        &mut stdin,
        &mut lines,
        55,
        "glyph_disconnect_server",
        json!({ "serverId": server_id }),
    )
    .await;
    assert!(!err);
    assert_eq!(disc["success"], true);

    // Unknown tool cleanly sets isError: true
    let (err, _) = call_tool(&mut stdin, &mut lines, 56, "glyph_does_not_exist", json!({})).await;
    assert!(err);

    drop(stdin);
    let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("child exit timeout")
        .expect("child wait");
    assert!(status.success());

    api_handle.shutdown().await;
    let _ = fs::remove_dir_all(&temp_dir);
}
