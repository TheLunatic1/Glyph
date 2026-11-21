# Glyph MCP & Local Loopback API Reference (36 Tools)

Glyph exposes a Bearer-token authenticated loopback HTTP server on `127.0.0.1:15354` (token stored at `~/.glyph/mcp_token`) paired with the standalone `glyph-mcp` stdio JSON-RPC 2.0 server (`crates/glyph-mcp`).

When `glyph-desktop` is closed and `GLYPH_NO_AUTOLAUNCH` is unset, `glyph-mcp` automatically launches `glyph-desktop` cleanly in the background (with Windows stdio handle inheritance disabled) and waits up to 15 seconds for port `15354` to become ready.

---

## Tool Catalog (36 Tools)

### Servers & Telemetry
| MCP Tool | HTTP Route | Required Parameters | Optional Parameters | Live Desktop UI Behavior |
|---|---|---|---|---|
| `glyph_list_servers` | `GET /api/servers` | `—` | `—` | Returns `{ id, name, host, username, port, os, groupId, tags }[]` |
| `glyph_connect_server` | `POST /api/connect` | `serverId` | `—` | Opens & focuses dedicated server window, detects OS |
| `glyph_disconnect_server` | `POST /api/disconnect` | `serverId` | `—` | Tears down SSH/VNC session and closes server window |
| `glyph_execute_command` | `POST /api/execute` | `serverId`, `command` | `—` | Switches to **Terminal**, opens `AI Agent` tab, streams command & output |
| `glyph_get_server_stats` | `POST /api/telemetry` | `serverId` | `—` | Switches to **Dashboard**, updates CPU/RAM/Disk/Net/Temp/GPU telemetry |

### SFTP File Browser & Transfer
| MCP Tool | HTTP Route | Required Parameters | Optional Parameters | Live Desktop UI Behavior |
|---|---|---|---|---|
| `glyph_read_file` | `POST /api/sftp/read` | `serverId`, `path` | `—` | Switches to **SFTP**, loads parent dir, opens file in editor |
| `glyph_write_file` | `POST /api/sftp/write` | `serverId`, `path`, `content` | `—` | Switches to **SFTP**, saves file, opens content in editor |
| `glyph_sftp_list` | `POST /api/sftp/list` | `serverId` | `path` *(default `/`)* | Switches to **SFTP**, updates directory listing + `[MCP Agent]` status |
| `glyph_sftp_mkdir` | `POST /api/sftp/mkdir` | `serverId`, `path` | `—` | Switches to **SFTP**, refreshes parent directory view |
| `glyph_sftp_rename` | `POST /api/sftp/rename` | `serverId`, `oldPath`, `newPath` | `—` | Switches to **SFTP**, refreshes destination directory |
| `glyph_sftp_chmod` | `POST /api/sftp/chmod` | `serverId`, `path`, `mode` | `—` | Switches to **SFTP**, accepts octal string (e.g. `"755"`) or int `<= 0o7777` |
| `glyph_sftp_delete` | `POST /api/sftp/delete` | `serverId`, `path` | `isDir` *(default `false`)* | Deletes file/dir & refreshes parent view *(rejects `/`, `.`, `..` with HTTP 400)* |
| `glyph_sftp_transfer` | `POST /api/sftp/transfer` | `sourceServerId`, `sourcePath`, `destServerId`, `destPath` | `—` | Streams directly SSH→SSH with zero local disk buffering; refreshes dest SFTP view |

### SSH Port Forwarding (`-L`, `-R`, `-D`)
| MCP Tool | HTTP Route | Required Parameters | Optional Parameters | Live Desktop UI Behavior |
|---|---|---|---|---|
| `glyph_list_tunnels` | `POST /api/tunnels/list` | `serverId` | `—` | Switches to **Tunnels** & refreshes active tunnel list |
| `glyph_start_tunnel` | `POST /api/tunnels/start` | `serverId`, `localPort`, `remotePort` | `remoteHost`, `protocol` | Selects **Local (-L)** toggle, binds port, refreshes Tunnels view |
| `glyph_start_remote_tunnel` | `POST /api/tunnels/start_remote` | `serverId`, `remotePort` *(0 allowed)*, `localPort` | `remoteBindAddr`, `localHost` | Selects **Remote (-R)** toggle, starts `tcpip-forward`, refreshes view |
| `glyph_start_socks_tunnel` | `POST /api/tunnels/start_socks` | `serverId` | `localPort` *(default `0`)* | Selects **Dynamic (-D)** toggle, starts RFC 1928 SOCKS5 proxy |
| `glyph_stop_tunnel` | `POST /api/tunnels/stop` | `serverId` | `tunnelId` or `localPort` | Stops by `tunnelId` (`L:`, `R:`, `D:`) or `localPort`, refreshes Tunnels view |

### Docker / Podman & Compose
| MCP Tool | HTTP Route | Required Parameters | Optional Parameters | Live Desktop UI Behavior |
|---|---|---|---|---|
| `glyph_list_containers` | `POST /api/containers/list` | `serverId` | `—` | Switches to **Containers** & refreshes container list |
| `glyph_container_logs` | `POST /api/containers/logs` | `serverId`, `containerId` | `tail` *(default `200`)* | Opens container detail tab directly on **Logs** sub-tab |
| `glyph_container_inspect` | `POST /api/containers/inspect` | `serverId`, `containerId` | `isRunning` *(default `true`)* | Opens container detail tab directly on **Inspect** sub-tab + stats |
| `glyph_container_action` | `POST /api/containers/action` | `serverId`, `containerId`, `action` | `—` | Runs `start` / `stop` / `restart` and refreshes **Containers** |
| `glyph_compose_action` | `POST /api/containers/compose` | `serverId`, `dir`, `action` | `—` | Runs `docker compose up -d` / `down`, opens Compose panel with output |

### Commands, Groups, Secrets & VNC
| MCP Tool | HTTP Route | Required Parameters | Optional Parameters | Live Desktop UI Behavior |
|---|---|---|---|---|
| `glyph_list_commands` | `POST /api/commands/list` | `serverId` | `—` | Switches to **Commands**; returns `{ "commands": [...] }` over HTTP |
| `glyph_add_command` | `POST /api/commands/add` | `serverId`, `name`, `cmd` | `—` | Saves command snippet & refreshes **Commands** tab |
| `glyph_remove_command` | `POST /api/commands/remove` | `serverId`, `id` | `—` | Removes command snippet by numeric ID & refreshes **Commands** |
| `glyph_list_snippet_packages` | `GET /api/snippet_packages` | `—` | `—` | Lists reusable global snippet bundles |
| `glyph_add_snippet_package` | `POST /api/snippet_packages/add` | `name` | `description`, `snippets[]` | Creates global snippet bundle & reloads palette/window caches |
| `glyph_list_groups` | `GET /api/groups` | `—` | `—` | Lists host groups + resolved subtree `serverIds[]` |
| `glyph_run_group_command` | `POST /api/groups/execute` | `groupId`, `command` | `—` | Broadcasts command across every group host with live `AI Agent` tabs |
| `glyph_list_secrets` | `POST /api/secrets/list` | `serverId` | `—` | Switches to **Secrets**; returns IDs & aliases ONLY *(never values)* |
| `glyph_add_secret` | `POST /api/secrets/add` | `serverId`, `name`, `value` | `—` | Encrypts & stores secret; refreshes **Secrets** tab |
| `glyph_list_ssh_keys` | `GET /api/ssh_keys` | `—` | `—` | Read-only key metadata, SHA-256 fingerprints, and OpenSSH public keys |
| `glyph_list_identities` | `GET /api/identities` | `—` | `—` | Read-only credential identity metadata *(never passwords/passphrases)* |
| `glyph_list_known_hosts` | `GET /api/known_hosts` | `—` | `—` | Read-only trusted SSH host key fingerprints |
| `glyph_open_vnc` | `POST /api/vnc/open` | `serverId` | `host`, `port`, `autoConnect` | Switches to **VNC** tab, fills host/port, optionally connects over SSH |

---

## Security Exclusions

To prevent unintended credential exposure or host filesystem access from LLM prompts, the following remain **UI-only** and are intentionally not exposed as MCP routes:
- Decrypting, exporting, or deleting secrets (`get_secret_value`, `export_secrets_encrypted`, `delete_secret`).
- Reading, generating, importing, deleting, or deploying SSH private keys.
- Adding/editing/deleting saved servers, identities, known hosts, app lock settings, or WebDAV/S3 sync configs.
- Local filesystem SFTP upload/download (`sftp_upload`, `sftp_download`), local `~/.ssh/config` import, or Local PTY / Serial / Telnet shells.
- VNC passwords (never transmitted over MCP).
