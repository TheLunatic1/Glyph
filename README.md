# Glyph v3 — Pure Rust SSH & Server Manager

**Made by TheLunatic1 (Salman Toha)**

Glyph is a cross-platform, 100% pure-Rust desktop SSH client, server telemetry suite, SFTP file explorer, Docker/Podman manager, VNC-over-SSH viewer, encrypted credential vault, and native Model Context Protocol (MCP) host for Windows, macOS, and Linux — replacing the legacy Electron/Node runtime while keeping 1:1 visual styling and full `.glyph` backup interoperability with [Glyph Mobile](https://github.com/TheLunatic1/glyph-app).

---

## Features

- **Host Groups, Tags & Reusable Identities:** Organize hosts into nested collapsible groups with cascading credential/jump-host/port/tag inheritance, reusable Credentials & Identities vault (updating an identity updates every inheriting host/group), and live search/group/tag filtering.
- **SSH Key Vault & TOFU Verification:** Generate/import Ed25519, RSA-4096, and ECDSA-P256 keys inside your OS keyring/Argon2id vault, deploy public keys to `~/.ssh/authorized_keys`, authenticate with Vault Key / Key File / Password / SSH Agent / ProxyJump, per-host Keepalive, custom environment variables, SSH Agent Forwarding (`-A`), startup commands, and TCP/UDP Port Knocking sequences, with Trust-On-First-Use (TOFU) host verification.
- **Quick Launch — Local Shell, Serial/UART & Telnet:** Launch native interactive local shells (`pwsh`/`powershell`/`bash`/`zsh`), hardware Serial/UART consoles (COM / `ttyUSB` / `ttyACM` with configurable baud/bits/parity/stop-bits), and RFC 854 Telnet clients in standalone desktop windows alongside SSH tabs.
- **Global Command Palette (`Ctrl+K` / `Cmd+K`):** Instant fuzzy jump across saved SSH hosts, active windows, groups, Local/Serial/Telnet quick-launch, identities, SSH keys, host importer, sync settings, and lock controls.
- **GPU-Accelerated Multi-Tab Split Terminal + Broadcast & Recording:** Powered by `alacritty_terminal` and `iced` canvas with split panes, Broadcast Input (`Off` / `Window Tabs` / `All Windows` with live warning banner), configurable regex terminal keyword highlighting rules, asciinema v2 `.cast` recording + per-host Auto-Log, `.log` plain text export, theme presets, search, quick snippet runner, and non-echoing Secret injection.
- **Dual-Pane SFTP, Remote-to-Remote Copy & Transfer Queue:** Side-by-side SFTP browser switching between `Remote <-> Local Computer` and `Remote <-> Peer Server`, breadcrumbs, sortable columns, drag-and-drop uploads, cancellable transfer queue drawer, clipboard copy/cut/paste, inline `chmod`/rename/delete, and remote text editor.
- **Global Snippet Packages, `{{var}}` Templates & Multi-Host Execution:** Save per-server commands or reusable cross-host Snippet Packages, auto-prompt for `{{var}}` template placeholders on execution, and run commands in parallel across an entire Host Group or all saved servers.
- **Port Forwarding (`-L`, `-R`, Dynamic SOCKS5 `-D`) & Saved Tunnel Profiles:** Bind Local, Remote, or RFC 1928 Dynamic SOCKS5 proxies, save recurring tunnel rules to server profiles, and toggle automatic startup on connect.
- **Host Importers & Exporters:** Import/preview hosts from OpenSSH `~/.ssh/config` (and export Vault back to SSH config), PuTTY `.reg`, CSV, or live cloud inventories (DigitalOcean, Hetzner Cloud, Tailscale), plus 100% AES-256-GCM `.glyph` backup interop with Glyph v2 and Glyph Android APK.
- **Master Vault Passcode & Zero-Knowledge E2EE Sync:** Optional Argon2id master passcode lock screen with configurable idle auto-lock timeout, and zero-knowledge client-encrypted sync over Local/Cloud folders or WebDAV (Nextcloud/OwnCloud) without storing unencrypted secrets or sync passphrases on disk.
- **5-Metric Live Telemetry, Docker/Podman & Pure-Rust VNC over SSH:** Real-time CPU/RAM/Disk/Net/Temp/GPU dashboards, Docker/Podman container & Compose controls, RFB 3.8 VNC-over-SSH remote desktop viewer, Bearer-protected loopback REST API (`127.0.0.1:15354`), standalone `glyph-mcp` stdio server (13 tools), and Mobile Companion APK QR/updater modal.

---

## Workspace Architecture (`glyph-rust`)

| Crate | Responsibility |
|---|---|
| `crates/glyph-core` | Vault crypto (`aes-gcm`, `scrypt`, `argon2`, `keyring`), SSH/ProxyJump (`russh` + `ring`) + Port Knocking + Agent Forwarding, telemetry parser, SFTP (including remote-to-remote streaming), `-L`/`-R`/`-D` SOCKS5 tunnels, Local PTY, Serial/UART, Telnet, SSH config/PuTTY/CSV/Cloud importers, E2EE Sync, Docker/Podman, MCP config manager, GitHub updater |
| `crates/glyph-terminal` | `alacritty_terminal` PTY state, regex keyword highlighting, asciinema `.cast` session recorder, plain `.log` exporter + GPU-accelerated `iced` terminal canvas widget |
| `crates/glyph-vnc` | Pure-Rust RFB 3.8 client + `iced` remote desktop viewport widget |
| `crates/glyph-api` | `axum` Bearer-protected loopback REST API (`127.0.0.1:15354`) shared by GUI and headless tests |
| `crates/glyph-mcp` | Standalone stdio JSON-RPC 2.0 MCP server (`glyph-mcp`) with all 13 `glyph_*` tools |
| `crates/glyph-desktop` | Cross-platform `iced` multi-window GUI, system tray, single-instance lock, and QR/Settings modals |

---

## Build & Verification

Glyph uses pure-Rust cryptography (`ring`, `aes-gcm`, `scrypt`, `argon2`, `des`) with **zero CMake, NASM, OpenSSL, or `aws-lc-sys` build dependencies**.

### Prerequisites

- Rust stable (`1.85+` / Rust 2021 edition)
- **Linux only:** `libx11-dev libxkbcommon-dev libwayland-dev libgtk-3-dev libxdo-dev libasound2-dev libdbus-1-dev libxcursor-dev libxi-dev libxinerama-dev libxrandr-dev libvulkan-dev pkg-config`

### Commands

```bash
# Lint workspace strictly
cargo clippy --workspace --all-targets -- -D warnings

# Run all unit, SSH/SFTP, VNC-over-SSH, REST API, and spawned MCP stdio E2E tests
cargo test --workspace

# Build release GUI + MCP server binaries
cargo build --workspace --release
```

---

## Automated CI & Releases

- **CI (`.github/workflows/ci.yml`):** Runs `cargo tree` dependency audit, strict `clippy -D warnings`, and full `cargo test --workspace` on every push/PR across `ubuntu-latest`, `windows-latest`, and `macos-latest`.
- **Release (`.github/workflows/release.yml`):** Reads `[workspace.package].version` from `Cargo.toml`, tags `v<version>`, compiles release `glyph-desktop` and `glyph-mcp` for Windows (`.exe` / `.zip`), macOS Universal (`.dmg` / `.zip`), and Linux (`.tar.gz` / `.AppImage`), and publishes directly to GitHub Releases.
