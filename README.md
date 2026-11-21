# Glyph v3 — Pure Rust SSH & Server Manager

**Made by TheLunatic1 (Salman Toha)**

Glyph is a cross-platform, 100% pure-Rust desktop SSH client, server telemetry suite, SFTP file explorer, Docker/Podman manager, VNC-over-SSH viewer, encrypted credential vault, and native Model Context Protocol (MCP) host for Windows, macOS, and Linux — replacing the legacy Electron/Node runtime while keeping 1:1 visual styling and full `.glyph` backup interoperability with [Glyph Mobile](https://github.com/TheLunatic1/glyph-app).

---

## Features

- **SSH Key Vault & TOFU Verification:** Generate/import Ed25519, RSA-4096, and ECDSA-P256 keys inside your OS keyring/Argon2id vault, deploy public keys to `~/.ssh/authorized_keys`, authenticate with Vault Key / Key File / Password / SSH Agent / ProxyJump, and verify host identities via Trust-On-First-Use (TOFU).
- **100% `.glyph` Mobile Interop:** Export and import encrypted server/secret bundles using identical `scrypt (N=16384, r=8, p=1)` + `AES-256-GCM` format compatible with Glyph v2 and Glyph Android APK.
- **5-Metric Live Telemetry & Modals:** Real-time CPU per-core breakdown, memory/swap, disk mounts, network RX/TX interfaces, temperature sensors, and NVIDIA/AMD GPU monitoring.
- **GPU-Accelerated Multi-Tab Split Terminal:** Powered by `alacritty_terminal` and `iced` canvas with split panes, themes, search, quick snippet runner, and non-echoing Secret injection.
- **SFTP File Explorer & Text Editor:** Breadcrumbs, sortable columns, drag-and-drop uploads, file download/transfer progress toasts, clipboard copy/cut/paste, inline `chmod`/rename/delete, and remote text editor.
- **Port Forwarding (`-L` & `-R`):** Manage Local and Remote SSH tunnels with live clickable `http`/`https` endpoints.
- **Docker / Podman Containers & Compose:** Container list, multi-tab Logs/Inspect/Resource Settings, live log tailing, and Docker Compose Up/Down.
- **Pure-Rust VNC over SSH:** RFB 3.8 client (`des` + `flate2` + `russh` `direct-tcpip`) with Fit/1:1 scaling, keyboard/mouse input, Ctrl+Alt+Del, and clipboard relay — zero plaintext VNC ports exposed.
- **Native Local REST API & `glyph-mcp`:** Bearer-authenticated loopback API on `127.0.0.1:15354` (`~/.glyph/mcp_token`) and standalone `glyph-mcp` stdio server exposing all 13 infrastructure tools for Antigravity IDE, Claude Desktop, Claude Code, Cursor, and VS Code.
- **Mobile Companion QR & Updater:** Live QR code downloader for `TheLunatic1/glyph-app` Android APK releases and built-in GitHub release update checking.

---

## Workspace Architecture (`glyph-rust`)

| Crate | Responsibility |
|---|---|
| `crates/glyph-core` | Vault crypto (`aes-gcm`, `scrypt`, `argon2`, `keyring`), SSH/ProxyJump (`russh` + `ring`), telemetry parser, SFTP, `-L`/`-R` tunnels, Docker/Podman, MCP config manager, GitHub updater |
| `crates/glyph-terminal` | `alacritty_terminal` PTY state + GPU-accelerated `iced` terminal canvas widget |
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
