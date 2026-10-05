# AGENTS.md — Glyph Developer & Architecture Documentation

## Project Overview & Philosophy
**Glyph** is a modern, secure, glassmorphic Electron + React SSH and server management application. It features integrated PTY terminal tabs, live SFTP browser & editor, real-time Docker container management, live CPU/RAM/Disk stats, tunnels, ZeroTier networking, master-password-encrypted server vault, selective backup export/import, and an embedded Model Context Protocol (MCP) server for local AI agents (Antigravity, Claude Desktop, Claude Code, Cursor, VS Code).

## Workspace & Codebase Tree
```
Glyph/
├── electron-builder.yml       # Electron builder packaging config & extraResources
├── package.json               # Scripts, dependencies, and app version (v2.7.6)
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
│   │       │   ├── SettingsModal.jsx   # AI Agent (MCP) & Mobile App config modal
│   │       │   ├── MobileAppModal.jsx  # Dynamic QR code & instant APK download modal
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

## Mobile Ecosystem (`glyph-app/`)
The mobile application (`D:\REPOSITORIES\glyph-app`) is a high-performance React Native / Expo application built for Android & iOS with 100% architectural and visual parity with Desktop Glyph:
1. **Server Schema Parity:** Direct alignment with Desktop vault schema: `{ id, name, host, username, port (default 22), password, privateKey, zerotier, os, status, metrics }`.
2. **ZeroTier Network Overlay:** First-class ZeroTier Network ID support with ZT Network status pill badge and mesh routing indications.
3. **OS Auto-Detection & Badges:** Remote OS is probed automatically upon SSH connection via `/etc/os-release` / `uname -s` and saved to the server in the vault, displaying distro-specific badges (Ubuntu, Debian, Alpine, CentOS, Arch, Fedora, macOS, Windows).
4. **Desktop Modal & Terminal Flow:** Add/Edit Server form directly ports Desktop fields with "Show Advanced Options" toggle. The Connecting Modal replicates Desktop's real-time terminal sequence with live ms counter and step logs.
5. **Connected Server Hub & Live Telemetry:** Feature parity across live Dashboard stats (CPU, RAM, Disk, GPU, and Network bandwidth) with full interactive hardware breakdown modals (Per-thread CPU load, Memory & Swap allocation, Storage & Mount partitions, Multi-vendor GPU driver setup & metrics, Network interface throughput), interactive Terminal with mobile accessory keys (ESC, TAB, CTRL, ALT, ^C, ^Z, ^D, |, /), SFTP file manager with remote code editor, Docker container cards with search & live logs, Commands catalog, Tunnels, and Secrets vault.
6. **Author Attribution & Open Source Documentation:** Author branding (`TheLunatic1 (Salman Toha)`) across app header and settings, alongside an embedded in-app markdown document viewer for Apache License 2.0, Contributing Guidelines (`CONTRIBUTING.md`), Code of Conduct (`CODE_OF_CONDUCT.md`), and Release Notes (`RELEASE_NOTES.md`).

## Build, Test & Run Workflow
### Desktop (Electron + React)
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

### Mobile (React Native / Expo)
```bash
# Bundle embedded JS for Android
npx expo export:embed --platform android --dev false --entry-file index.js --bundle-output android/app/src/main/assets/index.android.bundle --assets-dest android/app/src/main/res

# Install & run on attached Android device / emulator
cd android && .\gradlew.bat installDebug
adb shell am start -n dev.glyph.mobile/.MainActivity
```
