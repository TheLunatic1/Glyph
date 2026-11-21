# AGENTS.md — Glyph v3 Pure-Rust Developer & Architecture Guide

## Project Overview & Philosophy
**Glyph v3** is a 100% pure-Rust, cross-platform (Windows, macOS, Linux) desktop SSH client, server telemetry suite, dual-pane SFTP browser/editor, Docker/Podman manager, VNC-over-SSH remote desktop viewer, encrypted credential vault, and 36-tool Model Context Protocol (MCP) host — replacing the Electron v2 runtime (archived on branch `legacy-electron`) while preserving full visual fidelity and `.glyph` vault interoperability with [Glyph Mobile](https://github.com/TheLunatic1/glyph-app).

## Workspace Layout
```
Glyph/
├── .github/workflows/
│   ├── ci.yml                # Cross-OS lint, dependency audit & test suite (Win/Mac/Linux)
│   ├── release.yml           # Auto tag & multi-OS binary packager (Cargo.toml version)
│   └── security-guard.yml    # Dropper/malware signature scanner
├── assets/
│   ├── fonts/                # Bundled IBM Plex Sans/Mono static TTFs
│   └── logo.png              # Application & tray icon
├── docs/
│   └── MCP.md                # Full 36-tool MCP & loopback HTTP API reference
└── crates/
    ├── glyph-core/           # Pure-Rust core domain engine & unit/SSH E2E tests
    │   └── src/
    │       ├── vault.rs      # Encrypted VaultManager (servers, groups, identities, keys, secrets, snippets, E2EE sync)
    │       ├── crypto.rs     # AES-256-GCM, Argon2id, Scrypt, OS keyring integration
    │       ├── ssh.rs        # russh + ring SSH, ProxyJump, TOFU, Agent Forwarding, Port Knocking
    │       ├── sftp.rs       # russh-sftp operations + zero-copy remote-to-remote streaming
    │       ├── tunnels.rs    # Local (-L), Remote (-R), and RFC 1928 Dynamic SOCKS5 (-D) tunnels
    │       ├── containers.rs # Docker/Podman & Compose CLI orchestration + parsers
    │       ├── telemetry.rs  # Multi-section Linux server metrics parser (CPU/RAM/Disk/Net/Temp/GPU)
    │       ├── local_pty.rs  # Local shell PTY launcher (`portable-pty`)
    │       ├── serial.rs     # Pure-Rust Serial/UART session (`serialport` default-features=false)
    │       ├── telnet.rs     # RFC 854 Telnet + IAC NAWS session
    │       ├── importers.rs  # OpenSSH config, PuTTY .reg, CSV importers & SSH config exporter
    │       ├── cloud_import.rs # DigitalOcean, Hetzner, Tailscale host importers
    │       ├── sync.rs       # Zero-knowledge folder & WebDAV encrypted vault sync
    │       ├── mcp_config.rs # 1-click MCP client config detector/writer
    │       └── updater.rs    # Semver comparison & GitHub Releases checker
    ├── glyph-terminal/       # alacritty_terminal emulator, regex highlights, asciinema .cast recorder, Iced canvas
    ├── glyph-vnc/            # Pure-Rust RFB 3.8 + DES VNC client & Iced viewport widget
    ├── glyph-api/            # Axum loopback REST API (127.0.0.1:15354) + ApiUiEvent bus + E2E tests
    ├── glyph-mcp/            # Standalone stdio JSON-RPC 2.0 MCP binary (auto-launches GUI, 36 tools) + E2E tests
    └── glyph-desktop/        # Multi-window Iced 0.14 GUI, tray, splash, OS icons, palette & modals
```

## Critical Architectural Rules & Invariants
1. **Zero C/CMake/OpenSSL Dependencies:** Do **NOT** introduce `openssl-sys`, `aws-lc-sys`, `cmake`, `nasm`, or `libudev` into `Cargo.lock`. Use `ring` + `rustls` (`rustls-tls-webpki-roots-no-provider`) and pure-Rust crates (`aes-gcm`, `argon2`, `scrypt`, `des`). CI enforces this via `cargo tree`.
2. **Single-Instance & Loopback REST API:** `glyph-desktop` enforces `SingleInstance::new("com.glyph.app.v3")` and binds Axum HTTP on `127.0.0.1:15354` (configurable via `GLYPH_API_PORT` for isolated tests) protected by a constant-time Bearer token in `~/.glyph/mcp_token` (`GLYPH_MCP_TOKEN_FILE` override in tests).
3. **Windows Auto-Launch Pipe Hygiene (`glyph-mcp`):** When `glyph-mcp` spawns `glyph-desktop.exe` on Windows, it clears `HANDLE_FLAG_INHERIT` on stdio handles (`disable_stdio_handle_inheritance`) and passes `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` (`0x0000_0208`). Never pass `CREATE_BREAKAWAY_FROM_JOB` (fails under restricted Job Objects) and never skip clearing pipe inheritance (otherwise MCP clients block on EOF).
4. **MCP Live UI Synchronization & Security Boundary:** Every mutating/visual `glyph_*` route emits typed `ApiUiEvent` events handled in `GlyphApp::handle_api_ui_event` so actions open windows, focus tabs, stream `AI Agent` output, open files in the SFTP editor, open container Logs/Inspect sub-tabs, and refresh views live. Keep decrypted secrets, private SSH keys, destructive vault edits, local host filesystem reads/writes, VNC passwords, and root SFTP deletion (`/` / `.` / `..`) excluded from MCP.
5. **macOS Cross-Arch Compatibility:** `release.yml` builds universal macOS binaries (`aarch64-apple-darwin` + `x86_64-apple-darwin` via `lipo`) — avoid target-only crates or inline assembly that breaks on either target.
6. **Legacy Mobile Interoperability:** Keep `.glyph` AES-256-GCM export/import schemas compatible with `TheLunatic1/glyph-app` and legacy Glyph v2.

## Build, Test & Verify Workflow
```bash
# Check formatting/lints strictly (zero warnings allowed)
cargo clippy --workspace --all-targets -- -D warnings

# Run all unit & loopback E2E suites (SSH, SFTP, VNC, REST API, MCP stdio)
cargo test --workspace

# Run desktop GUI from source
cargo run -p glyph-desktop

# Build release binaries
cargo build --workspace --release
```

### Windows Low-Memory Cargo Guardrail
On Windows hosts with tight memory when compiling debug artifacts alongside the browser/IDE:
```powershell
$env:CARGO_PROFILE_DEV_DEBUG="0"
cargo clippy --workspace --all-targets -j 1 -- -D warnings
cargo test --workspace -j 1
cargo build --workspace --release -j 2
```
