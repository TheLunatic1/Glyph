## v3.0.1 — Window Lifecycle & Resizing, Live Activity Logger, Focus-Safe MCP & Updater Fix

> **Upgrading from Windows `v3.0.0`?** Because `v3.0.0` contained a Windows self-updater launcher bug (`0x0800_0008`), install [`Glyph-Setup-3.0.1.exe`](https://github.com/TheLunatic1/Glyph/releases/download/v3.0.1/Glyph-Setup-3.0.1.exe) once directly. Starting from `v3.0.1`, one-click in-app **Restart & Install** updates work automatically end-to-end.

### What's New & Fixed
- **Independent Manager Window Close:** Closing the Glyph Manager window while child Server or Quick Terminal windows are open now cleanly closes only the Manager window without interrupting active child sessions (and reopens easily from the system tray or when no windows/tray remain).
- **Full 8-Direction Window Resizing & Cross-Platform Titlebars:**
  - Frameless Manager, Server, and Quick Terminal windows on Windows & Linux now support full edge and corner drag resizing (`N`, `S`, `E`, `W`, `NE`, `NW`, `SE`, `SW`) with resize cursors plus double-click titlebar maximize.
  - macOS builds now use transparent full-size content titlebars with proper traffic-light spacing so native macOS resizing and snapping work natively.
- **In-App Activity Logger (`Logs` button / `Ctrl+Shift+L`):**
  - Live chronological feed of **Success**, **Info**, **Warning**, and **Error** events across SSH, SFTP, Tunnels, Docker/Compose, VNC, Vault, Updater, and MCP tool calls.
  - Filter by severity or keyword, copy formatted logs to clipboard, view unread warning/error counts right in the titlebar, and persist sanitized events to `~/.glyph/activity.log`.
- **Zero Focus Stealing for AI / MCP Work:**
  - `glyph-mcp` background operations (and MCP auto-launch) no longer pop Glyph windows on top of your browser, video player, or active app.
  - Background MCP server sessions appear under **Active Sessions** in Manager and in the `Ctrl+K` Command Palette so you can click to focus them on demand.
- **Fixed Windows In-App Auto-Updater & NSIS Upgrade Swap:**
  - Launches downloaded `Glyph-Setup-*.exe` installers directly with `CREATE_NEW_PROCESS_GROUP` (`0x0000_0200`) instead of passing mutually exclusive Win32 creation flags (`CREATE_NO_WINDOW | DETACHED_PROCESS`) to `powershell.exe`, and uses valid `CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP` (`0x0800_0200`) with `~/.glyph/updater.log` fallback logging for standalone/zip updates.
  - Cleanly closes any running `Glyph.exe` during both interactive and silent upgrades and rotates locked `glyph-mcp.exe` handles via `glyph-mcp.exe.old.$HWNDPARENT`.

---

## v3.0.0 — Pure Rust Rewrite, Full Termius Parity, Native Auto-Updater & 36-Tool Visual MCP

> **Upgrading from Glyph v2 (Electron)?** Glyph **v3.0.0** is now available as a **100% Pure Rust** native application — smaller binary size, instant startup, native GPU terminal rendering, full `.glyph` vault & SSH key compatibility, and an interactive **in-app auto-updater**. Click **Download & Install** in your update prompt or install [`Glyph-Setup-3.0.0.exe`](https://github.com/TheLunatic1/Glyph/releases/tag/v3.0.0) directly.

### Highlights
- **100% Pure Rust Desktop & Engine:** Rebuilt from the ground up with `iced`, `alacritty_terminal`, `russh` (`ring`), and `axum` — zero Node.js, zero Chromium, and zero CMake/OpenSSL build dependencies across Windows, macOS Universal (`arm64` + `x64`), and Linux.
- **Native In-App Auto-Updater + v2 Upgrade Bridge:**
  - Silent startup release check + manual check under **Settings -> Updates**, live top notification banner, formatted release notes dialog, streaming progress bar (`%` and speed), and **SHA-256** verification against `SHA256SUMS.txt`.
  - One-click **Restart & Install** across Windows (NSIS `Glyph-Setup`, standalone `.exe`, `.zip` bundle), Linux (`.AppImage` and `.tar.gz`), and macOS (Universal `.app` `.zip` and `.dmg`).
  - Publishes `latest.yml`, `latest-mac.yml`, and `latest-linux.yml` manifests so legacy Electron v2.7.6 users automatically receive update notifications and can upgrade directly to Rust v3.0.0.
  - **macOS Universal Gatekeeper note:** Ad-hoc signed without paid Apple notarization — drag `Glyph.app` to `/Applications`, then open via **System Settings -> Privacy & Security -> Open Anyway** or `xattr -cr /Applications/Glyph.app`.
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
