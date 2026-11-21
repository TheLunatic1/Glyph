use glyph_api::{
    ensure_mcp_token, start_local_api, ApiHostKeyPolicy, LocalVaultApiController,
};
use glyph_core::models::{AuthMethod, ServerConfigInput};
use glyph_core::ssh::SshSession;
use glyph_core::vault::VaultManager;
use russh::keys::ssh_key::{getrandom::SysRng, rand_core::UnwrapErr};
use russh::keys::{Algorithm, PrivateKey};
use russh::server::{self, Auth, ChannelOpenHandle, Config, Handler, Msg, Session};
use russh::{Channel, ChannelId};
use russh_sftp::protocol::{
    Data, FileAttributes, Handle, OpenFlags, Status, StatusCode as SftpStatus, Version,
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
        let handle = format!("m-{}", self.next_id);
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
    let server_id = vault
        .add_server(ServerConfigInput {
            name: "MCP Test Box".into(),
            host: "127.0.0.1".into(),
            username: "mcpuser".into(),
            port: ssh_addr.port(),
            password: Some("mcppass".into()),
            auth_method: AuthMethod::Password,
            ..Default::default()
        })
        .expect("add server");

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

    // 3. tools/list asserts all 13 exact tool names
    let list = rpc_call(
        &mut stdin,
        &mut lines,
        json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list" }),
    )
    .await;
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 13);
    let names: BTreeSet<&str> = tools
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    for expected in [
        "glyph_list_servers",
        "glyph_connect_server",
        "glyph_execute_command",
        "glyph_read_file",
        "glyph_write_file",
        "glyph_start_tunnel",
        "glyph_stop_tunnel",
        "glyph_list_secrets",
        "glyph_add_secret",
        "glyph_list_containers",
        "glyph_list_commands",
        "glyph_add_command",
        "glyph_remove_command",
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

    // Unknown tool cleanly sets isError: true
    let (err, _) = call_tool(&mut stdin, &mut lines, 23, "glyph_does_not_exist", json!({})).await;
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
