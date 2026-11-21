## v3.0.0 — Pure Rust Rewrite, Full Termius Parity & 36-Tool Visual MCP

### Highlights
- **100% Pure Rust Desktop & Engine:** Rebuilt from the ground up with `iced`, `alacritty_terminal`, `russh` (`ring`), and `axum` — zero Node.js, zero Chromium, and zero CMake/OpenSSL build dependencies across Windows, macOS Universal (`arm64` + `x64`), and Linux.
- **Full Visual Parity + Termius-Grade Terminal & Connectivity:**
  - Bundled **IBM Plex Sans/Mono** typography, **Lucide** icon font, crisp frameless window controls, animated splash screen, and automatic distro OS icons (including Oracle Linux `ol`).
  - Split terminal panes, Broadcast Input (`Window Tabs` / `All Windows`), regex keyword highlighting, and asciinema v2 `.cast` + `.log` session recording.
  - Local Shell (`pwsh` / `bash` / `zsh`), Serial/UART, Telnet, Remote forwarding (`-R`), Dynamic SOCKS5 proxies (`-D`), SSH Agent Forwarding (`-A`), and TCP/UDP Port Knocking.
  - Nested Host Groups with cascading inheritance, reusable Credentials & Identities, Snippet Packages with `{{var}}` templates, multi-host group execution, and OpenSSH/PuTTY/CSV/DigitalOcean/Hetzner/Tailscale importers.
  - Dual-pane SFTP (`Remote <-> Local` and direct `Remote <-> Peer Server` streaming) with cancellable transfer queue, Docker/Podman + Compose manager, pure-Rust RFB 3.8 VNC-over-SSH viewer, Argon2id Master Lock, and Zero-Knowledge E2EE folder/WebDAV sync.
- **36 Security-Audited MCP Tools with Live UI Visualization:**
  - Native standalone `glyph-mcp` binary with non-blocking Windows auto-launch of `glyph-desktop` onto your active desktop.
  - Live `AI Agent` terminal tab, live SFTP editor & directory refreshes, Dashboard telemetry sync, Tunnel direction switching, Container Logs/Inspect sub-tabs & Compose output, Group broadcast execution, and VNC tab control — while deliberately excluding secret decryption, private key export, destructive host deletion, and root filesystem deletion.

*(The previous Electron/React v2.7.6 codebase remains archived on the [`legacy-electron`](https://github.com/TheLunatic1/Glyph/tree/legacy-electron) branch.)*
