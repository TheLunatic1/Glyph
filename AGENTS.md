# AGENTS.md — Glyph Developer & Architecture Documentation

## Project Overview & Philosophy
**Glyph** is a modern, secure, glassmorphic Electron + React SSH and server management application. It features integrated PTY terminal tabs, live SFTP browser & editor, real-time Docker container management, live CPU/RAM/Disk stats, tunnels, ZeroTier networking, master-password-encrypted server vault, selective backup export/import, and an embedded Model Context Protocol (MCP) server for local AI agents (Antigravity, Claude Desktop, Claude Code, Cursor, VS Code).

## Workspace & Codebase Tree
```
Glyph/
├── electron-builder.yml       # Electron builder packaging config & extraResources
├── package.json               # Scripts, dependencies, and app version (v2.7.5)
├── resources/
│   ├── logo.png               # App & system tray icon
│   └── mcp.js                 # Standalone bundled MCP server bundle for production
├── scripts/
│   └── bundle-mcp.cjs         # Standalone MCP bundler using esbuild
├── src/
│   ├── main/
│   │   ├── index.js           # Electron main process, tray, window routes, single-instance lock
│   │   ├── api.js             # Local token-authenticated HTTP API (port 15354) for MCP
│   │   ├── sshManager.js      # SSH connection lifecycle, PTY stream, SFTP client
│   │   ├── vault.js           # Server credential vault with AES/safeStorage encryption
│   │   ├── secretsVault.js    # Per-server secrets manager
│   │   ├── updater.js         # Auto-updater lifecycle
│   │   └── cryptoUtil.js      # AES-256-GCM encryption/decryption utilities
│   ├── preload/
│   │   └── index.js           # Electron contextBridge IPC definitions (`window.api`)
│   ├── renderer/
│   │   ├── index.html         # HTML entry point
│   │   └── src/
│   │       ├── App.jsx        # Root component, routing, connection overlays, modals
│   │       ├── assets/        # Visual assets and logo
│   │       ├── components/    # Reusable UI components
│   │       │   ├── TitleBar.jsx        # Drag title bar with left attribution
│   │       │   ├── SplashScreen.jsx    # Smooth launch splash screen
│   │       │   ├── ExportModal.jsx     # Selective server export modal
│   │       │   ├── ImportModal.jsx     # Selective server import modal
│   │       │   ├── SettingsModal.jsx   # AI Agent (MCP) multi-client config modal
│   │       │   ├── Sidebar.jsx         # Server view navigation
│   │       │   ├── OsLogo.jsx          # OS detection badges
│   │       │   └── UpdateModal.jsx     # In-app update manager
│   │       └── pages/         # Connected server views
│   │           ├── Dashboard.jsx       # CPU, RAM, Disk, Uptime metrics
│   │           ├── Terminal.jsx        # xterm.js PTY terminal tabs
│   │           ├── SFTP.jsx            # Remote file browser & editor
│   │           ├── Containers.jsx      # Docker management with search & compose
│   │           ├── Commands.jsx        # Saved snippet launcher
│   │           ├── Secrets.jsx         # Server secret injection
│   │           └── Tunnels.jsx         # Local/remote port forwarding
│   └── mcp/
│       └── index.js           # MCP server implementation with auto-launch capability
```

## Critical Architectural Rules & Invariants
1. **Single Instance Lock:** Always enforce `app.requestSingleInstanceLock()` in `src/main/index.js` to ensure subsequent launches restore and focus the existing main window from the tray without port 15354 collisions.
2. **Local MCP API Protocol:** The MCP backend runs an Express server on `127.0.0.1:15354` authenticated with a per-session token stored in `~/.glyph/mcp_token`.
3. **Safe Storage & Encrypted Backups:** Passwords in the vault use Electron `safeStorage` locally, and `.glyph` export files are encrypted with user-provided master passwords via AES-256-GCM.
4. **Window Isolation:** Each connected server runs in a dedicated `BrowserWindow` keyed by `windowRoutes` and `sshManagers` maps.

## Build, Test & Run Workflow
```bash
# Install dependencies
npm install

# Run in development mode (hot-reloading)
npm run dev

# Build production bundle (auto-bundles MCP server + Vite)
npm run build

# Package installers (NSIS / portable for Windows)
npm run dist
```
