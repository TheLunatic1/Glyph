use anyhow::{anyhow, Context, Result};
use directories::BaseDirs;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[tokio::main]
async fn main() -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();
    let stdout = std::io::stdout();

    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(err) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": format!("Parse error: {err}") }
                });
                write_json_line(&stdout, &err_resp)?;
                continue;
            }
        };

        // Notifications do not have an `id` and must not receive a response.
        let Some(id) = req.id.clone() else {
            continue;
        };

        let response = handle_request(req.method, req.params, id).await;
        write_json_line(&stdout, &response)?;
    }

    Ok(())
}

fn write_json_line(stdout: &std::io::Stdout, value: &Value) -> Result<()> {
    let serialized = serde_json::to_string(value)?;
    let mut lock = stdout.lock();
    lock.write_all(serialized.as_bytes())?;
    lock.write_all(b"\n")?;
    lock.flush()?;
    Ok(())
}

async fn handle_request(method: String, params: Value, id: Value) -> Value {
    match method.as_str() {
        "initialize" => {
            let protocol_version = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("2024-11-05");
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": protocol_version,
                    "capabilities": {
                        "tools": {
                            "listChanged": false
                        }
                    },
                    "serverInfo": {
                        "name": "glyph-ssh-manager",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            })
        }
        "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": tool_definitions()
            }
        }),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            match execute_tool(name, &args).await {
                Ok(text) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }
                }),
                Err(err) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": format!("Glyph API Error: {err}") }],
                        "isError": true
                    }
                }),
            }
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {method}")
            }
        }),
    }
}

async fn execute_tool(name: &str, args: &Value) -> Result<String> {
    let val = match name {
        "glyph_list_servers" => call_glyph_api("GET", "/api/servers", None).await?,
        "glyph_connect_server" => {
            let server_id = req_str(args, "serverId")?;
            call_glyph_api("POST", "/api/connect", Some(json!({ "serverId": server_id }))).await?
        }
        "glyph_execute_command" => {
            let server_id = req_str(args, "serverId")?;
            let command = req_str(args, "command")?;
            call_glyph_api(
                "POST",
                "/api/execute",
                Some(json!({ "serverId": server_id, "command": command })),
            )
            .await?
        }
        "glyph_read_file" => {
            let server_id = req_str(args, "serverId")?;
            let path = req_str(args, "path")?;
            call_glyph_api(
                "POST",
                "/api/sftp/read",
                Some(json!({ "serverId": server_id, "path": path })),
            )
            .await?
        }
        "glyph_write_file" => {
            let server_id = req_str(args, "serverId")?;
            let path = req_str(args, "path")?;
            let content = req_str_allow_empty(args, "content")?;
            call_glyph_api(
                "POST",
                "/api/sftp/write",
                Some(json!({ "serverId": server_id, "path": path, "content": content })),
            )
            .await?
        }
        "glyph_start_tunnel" => {
            let server_id = req_str(args, "serverId")?;
            let local_port = args.get("localPort").and_then(Value::as_u64).unwrap_or(0);
            let remote_host = args
                .get("remoteHost")
                .and_then(Value::as_str)
                .unwrap_or("127.0.0.1");
            let remote_port = args
                .get("remotePort")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow!("Missing required integer parameter: remotePort"))?;
            let protocol = args
                .get("protocol")
                .and_then(Value::as_str)
                .unwrap_or("TCP");
            call_glyph_api(
                "POST",
                "/api/tunnels/start",
                Some(json!({
                    "serverId": server_id,
                    "localPort": local_port,
                    "remoteHost": remote_host,
                    "remotePort": remote_port,
                    "protocol": protocol,
                })),
            )
            .await?
        }
        "glyph_stop_tunnel" => {
            let server_id = req_str(args, "serverId")?;
            let local_port = args
                .get("localPort")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow!("Missing required integer parameter: localPort"))?;
            call_glyph_api(
                "POST",
                "/api/tunnels/stop",
                Some(json!({ "serverId": server_id, "localPort": local_port })),
            )
            .await?
        }
        "glyph_list_secrets" => {
            let server_id = req_str(args, "serverId")?;
            call_glyph_api("POST", "/api/secrets/list", Some(json!({ "serverId": server_id })))
                .await?
        }
        "glyph_add_secret" => {
            let server_id = req_str(args, "serverId")?;
            let secret_name = req_str(args, "name")?;
            let value = req_str_allow_empty(args, "value")?;
            call_glyph_api(
                "POST",
                "/api/secrets/add",
                Some(json!({ "serverId": server_id, "name": secret_name, "value": value })),
            )
            .await?
        }
        "glyph_list_containers" => {
            let server_id = req_str(args, "serverId")?;
            call_glyph_api(
                "POST",
                "/api/containers/list",
                Some(json!({ "serverId": server_id })),
            )
            .await?
        }
        "glyph_list_commands" => {
            let server_id = req_str(args, "serverId")?;
            call_glyph_api(
                "POST",
                "/api/commands/list",
                Some(json!({ "serverId": server_id })),
            )
            .await?
        }
        "glyph_add_command" => {
            let server_id = req_str(args, "serverId")?;
            let cmd_name = req_str(args, "name")?;
            let cmd = req_str(args, "cmd")?;
            call_glyph_api(
                "POST",
                "/api/commands/add",
                Some(json!({ "serverId": server_id, "name": cmd_name, "cmd": cmd })),
            )
            .await?
        }
        "glyph_remove_command" => {
            let server_id = req_str(args, "serverId")?;
            let cmd_id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow!("Missing required integer parameter: id"))?;
            call_glyph_api(
                "POST",
                "/api/commands/remove",
                Some(json!({ "serverId": server_id, "id": cmd_id })),
            )
            .await?
        }
        other => return Err(anyhow!("Unknown tool: {other}")),
    };

    Ok(serde_json::to_string_pretty(&val)?)
}

fn req_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    let val = args
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing required string parameter: {key}"))?;
    if val.trim().is_empty() {
        return Err(anyhow!("Parameter `{key}` cannot be empty"));
    }
    Ok(val)
}

fn req_str_allow_empty<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing required string parameter: {key}"))
}

fn api_host_port() -> String {
    let raw = std::env::var("GLYPH_API_URL").unwrap_or_else(|_| "http://127.0.0.1:15354".into());
    raw.trim()
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string()
}

fn token_file_path() -> Result<PathBuf> {
    if let Ok(override_path) = std::env::var("GLYPH_MCP_TOKEN_FILE") {
        let trimmed = override_path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    if let Some(base) = BaseDirs::new() {
        return Ok(base.home_dir().join(".glyph").join("mcp_token"));
    }
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        return Ok(PathBuf::from(home).join(".glyph").join("mcp_token"));
    }
    Err(anyhow!("Unable to locate ~/.glyph/mcp_token"))
}

fn read_mcp_token() -> Result<String> {
    let path = token_file_path()?;
    let content = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "Failed to read Glyph MCP token at {} (is Glyph Desktop running?)",
            path.display()
        )
    })?;
    let trimmed = content.trim().to_string();
    if trimmed.is_empty() {
        return Err(anyhow!("Glyph MCP token file at {} is empty", path.display()));
    }
    Ok(trimmed)
}

async fn call_glyph_api(method: &str, path: &str, body: Option<Value>) -> Result<Value> {
    let host_port = api_host_port();
    let stream = match TcpStream::connect(&host_port).await {
        Ok(s) => s,
        Err(err) if err.kind() == ErrorKind::ConnectionRefused => {
            ensure_desktop_running(&host_port).await?;
            TcpStream::connect(&host_port)
                .await
                .with_context(|| format!("Failed to connect to Glyph Local API at {host_port}"))?
        }
        Err(err) => {
            return Err(anyhow!(
                "Failed to connect to Glyph Local API at {host_port}: {err}"
            ))
        }
    };

    let token = read_mcp_token()?;
    send_http_json(stream, &host_port, &token, method, path, body).await
}

async fn ensure_desktop_running(host_port: &str) -> Result<()> {
    if std::env::var("GLYPH_NO_AUTOLAUNCH").as_deref() == Ok("1") {
        return Err(anyhow!(
            "Glyph Desktop API at {host_port} is not running (auto-launch disabled via GLYPH_NO_AUTOLAUNCH=1)"
        ));
    }

    let exe = std::env::current_exe().context("Cannot resolve glyph-mcp executable path")?;
    let dir = exe
        .parent()
        .ok_or_else(|| anyhow!("Cannot resolve parent directory of glyph-mcp"))?;
    let candidates: &[&str] = if cfg!(windows) {
        &["glyph-desktop.exe", "Glyph.exe"]
    } else if cfg!(target_os = "macos") {
        &["glyph-desktop", "Glyph", "../MacOS/Glyph"]
    } else {
        &["glyph-desktop", "Glyph"]
    };

    let mut launched = false;
    for name in candidates {
        let candidate = dir.join(name);
        if candidate.exists() {
            eprintln!("glyph-mcp: auto-launching {}", candidate.display());
            let mut cmd = Command::new(&candidate);
            cmd.stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0000_0200);
            }
            cmd.spawn()
                .with_context(|| format!("Failed to launch {}", candidate.display()))?;
            launched = true;
            break;
        }
    }

    if !launched {
        return Err(anyhow!(
            "Glyph Desktop is not running at {host_port} and no sibling desktop executable was found in {}",
            dir.display()
        ));
    }

    let token_path = token_file_path()?;
    for _ in 0..60 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        if token_path.exists() && TcpStream::connect(host_port).await.is_ok() {
            return Ok(());
        }
    }

    Err(anyhow!(
        "Timed out waiting 15s for Glyph Desktop Local API on {host_port}"
    ))
}

async fn send_http_json(
    mut stream: TcpStream,
    host_port: &str,
    token: &str,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> Result<Value> {
    let payload = body
        .map(|v| serde_json::to_vec(&v))
        .transpose()?
        .unwrap_or_default();

    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host_port}\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n"
    );
    if !payload.is_empty() {
        req.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            payload.len()
        ));
    }
    req.push_str("\r\n");

    stream.write_all(req.as_bytes()).await?;
    if !payload.is_empty() {
        stream.write_all(&payload).await?;
    }
    stream.flush().await?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await?;
    let (status, resp_body) = parse_http_response(&raw)?;
    let text = String::from_utf8_lossy(&resp_body);

    if !(200..300).contains(&status) {
        if let Ok(err_json) = serde_json::from_str::<Value>(&text) {
            if let Some(msg) = err_json.get("error").and_then(Value::as_str) {
                return Err(anyhow!("HTTP {status}: {msg}"));
            }
        }
        return Err(anyhow!("HTTP {status}: {}", text.trim()));
    }

    if text.trim().is_empty() {
        return Ok(json!({ "success": true }));
    }
    serde_json::from_str(&text)
        .with_context(|| format!("Invalid JSON response from Glyph API: {text}"))
}

fn parse_http_response(raw: &[u8]) -> Result<(u16, Vec<u8>)> {
    let sep = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| anyhow!("Malformed HTTP response from Glyph API"))?;
    let header_text = String::from_utf8_lossy(&raw[..sep]);
    let body_bytes = &raw[sep + 4..];

    let mut lines = header_text.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| anyhow!("Missing HTTP status line"))?;
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow!("Invalid HTTP status line: {status_line}"))?;

    let mut is_chunked = false;
    let mut content_length: Option<usize> = None;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim();
            if key == "transfer-encoding" && val.to_ascii_lowercase().contains("chunked") {
                is_chunked = true;
            } else if key == "content-length" {
                content_length = val.parse().ok();
            }
        }
    }

    let decoded = if is_chunked {
        decode_chunked(body_bytes)?
    } else if let Some(len) = content_length {
        body_bytes[..body_bytes.len().min(len)].to_vec()
    } else {
        body_bytes.to_vec()
    };

    Ok((status, decoded))
}

fn decode_chunked(mut data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    while let Some(pos) = data.windows(2).position(|w| w == b"\r\n") {
        let size_str = std::str::from_utf8(&data[..pos])
            .context("Invalid chunked encoding size")?
            .split(';')
            .next()
            .unwrap_or("")
            .trim();
        let size = usize::from_str_radix(size_str, 16)
            .with_context(|| format!("Invalid hex chunk size: {size_str}"))?;
        data = &data[pos + 2..];
        if size == 0 {
            break;
        }
        if data.len() < size + 2 {
            return Err(anyhow!("Truncated chunked HTTP body"));
        }
        out.extend_from_slice(&data[..size]);
        data = &data[size + 2..];
    }
    Ok(out)
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "glyph_list_servers",
            "description": "List all saved SSH servers available in Glyph.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "glyph_connect_server",
            "description": "Connect to a saved SSH server in Glyph and open or activate its session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "serverId": { "type": "string", "description": "Server ID from glyph_list_servers" }
                },
                "required": ["serverId"]
            }
        }),
        json!({
            "name": "glyph_execute_command",
            "description": "Execute a shell command over SSH on the target server and return stdout/stderr output.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "serverId": { "type": "string", "description": "Server ID from glyph_list_servers" },
                    "command": { "type": "string", "description": "Shell command to execute" }
                },
                "required": ["serverId", "command"]
            }
        }),
        json!({
            "name": "glyph_read_file",
            "description": "Read a text file from the remote server over SFTP.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "serverId": { "type": "string", "description": "Server ID from glyph_list_servers" },
                    "path": { "type": "string", "description": "Remote file path" }
                },
                "required": ["serverId", "path"]
            }
        }),
        json!({
            "name": "glyph_write_file",
            "description": "Write UTF-8 text content to a file on the remote server over SFTP.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "serverId": { "type": "string", "description": "Server ID from glyph_list_servers" },
                    "path": { "type": "string", "description": "Remote file path" },
                    "content": { "type": "string", "description": "File content to write" }
                },
                "required": ["serverId", "path", "content"]
            }
        }),
        json!({
            "name": "glyph_start_tunnel",
            "description": "Start a local SSH port-forwarding tunnel to a remote host and port.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "serverId": { "type": "string", "description": "Server ID from glyph_list_servers" },
                    "localPort": { "type": "integer", "description": "Local TCP port on 127.0.0.1 (0 for ephemeral)" },
                    "remoteHost": { "type": "string", "description": "Destination host reachable from remote server (default 127.0.0.1)" },
                    "remotePort": { "type": "integer", "description": "Destination port on remote host" },
                    "protocol": { "type": "string", "description": "Tunnel protocol label (default TCP)" }
                },
                "required": ["serverId", "localPort", "remotePort"]
            }
        }),
        json!({
            "name": "glyph_stop_tunnel",
            "description": "Stop an active SSH port-forwarding tunnel by its local port.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "serverId": { "type": "string", "description": "Server ID from glyph_list_servers" },
                    "localPort": { "type": "integer", "description": "Bound local TCP port to close" }
                },
                "required": ["serverId", "localPort"]
            }
        }),
        json!({
            "name": "glyph_list_secrets",
            "description": "List saved secret names and IDs (without decrypting values) for a server.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "serverId": { "type": "string", "description": "Server ID from glyph_list_servers" }
                },
                "required": ["serverId"]
            }
        }),
        json!({
            "name": "glyph_add_secret",
            "description": "Encrypt and store a new secret for a server in Glyph's Secrets Vault.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "serverId": { "type": "string", "description": "Server ID from glyph_list_servers" },
                    "name": { "type": "string", "description": "Secret alias" },
                    "value": { "type": "string", "description": "Secret value" }
                },
                "required": ["serverId", "name", "value"]
            }
        }),
        json!({
            "name": "glyph_list_containers",
            "description": "List Docker/Podman containers on the remote server.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "serverId": { "type": "string", "description": "Server ID from glyph_list_servers" }
                },
                "required": ["serverId"]
            }
        }),
        json!({
            "name": "glyph_list_commands",
            "description": "List saved command snippets for a server sorted by usage count.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "serverId": { "type": "string", "description": "Server ID from glyph_list_servers" }
                },
                "required": ["serverId"]
            }
        }),
        json!({
            "name": "glyph_add_command",
            "description": "Add a reusable command snippet to a server in Glyph.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "serverId": { "type": "string", "description": "Server ID from glyph_list_servers" },
                    "name": { "type": "string", "description": "Snippet title" },
                    "cmd": { "type": "string", "description": "Shell command" }
                },
                "required": ["serverId", "name", "cmd"]
            }
        }),
        json!({
            "name": "glyph_remove_command",
            "description": "Delete a saved command snippet by numeric ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "serverId": { "type": "string", "description": "Server ID from glyph_list_servers" },
                    "id": { "type": "integer", "description": "Command snippet ID" }
                },
                "required": ["serverId", "id"]
            }
        }),
    ]
}
