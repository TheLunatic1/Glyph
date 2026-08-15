# Glyph

Glyph is a modern, high-performance SSH client and server management application built with Electron and React. Designed for power users, developers, and system administrators, Glyph combines a beautiful user interface with powerful networking, automation, and AI-agent features.

---

## Table of Contents

1. [Features](#features)
2. [Installation](#installation)
3. [Architecture](#architecture)
4. [Security](#security)
5. [AI Agent Setup (MCP)](#ai-agent-setup-mcp)
6. [Development](#development)

---

## Features

### Comprehensive Server Management
* **Secure Vault:** Store server credentials and configurations in a secure, encrypted vault.
* **Master Password Encryption:** Optionally encrypt your entire server database with a master password.
* **Quick Connect:** Seamlessly connect to your saved environments with a single click.

### Advanced Terminal & UI
* **Multi-Tab Interface:** Manage multiple SSH sessions concurrently with a clean tabbed layout.
* **Customizable Themes:** Switch between various syntax and terminal color themes (Dracula, Monokai, One Dark, Solarized).
* **Live System Dashboard:** View real-time metrics including CPU usage, RAM, and Disk space for connected servers.

### Next-Level Connectivity
* **Universal Protocol Tunneling:** Forward any TCP or UDP traffic from your local machine to remote destinations.
* **Built-in ZeroTier Integration:** Connect directly to ZeroTier virtual networks from within the application.
* **Visual SFTP Client:** Transfer, read, and edit files on remote servers using the integrated file browser.

### Automation & AI
* **Command Snippets:** Save and execute frequently used shell commands.
* **Secret Injection:** Store environment variables and secrets, securely injecting them into your terminal when needed.
* **In-App AI Agent Setup:** Configure any MCP-compatible AI assistant to control your servers — directly from the home screen, no manual config editing required.
* **Agentic AI Support (MCP):** Once connected, AI agents can list servers, run commands, manage files via SFTP, control tunnels, manage secrets, and more.

---

## Installation

### Prerequisites
* Node.js v18 or newer
* npm

### Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/TheLunatic1/Glyph.git
   cd Glyph
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Run the application in development mode:
   ```bash
   npm run dev
   ```

### Building for Production

```bash
npm run build   # bundles MCP server + compiles app
npm run dist    # packages a distributable installer
```

---

## Architecture

Glyph utilises a modern stack to ensure high performance and maintainability:

* **Frontend:** React 18, Tailwind CSS, Lucide React (Icons).
* **Backend (Main Process):** Electron, Node.js, `ssh2` for core networking.
* **Terminal Emulator:** xterm.js for high-fidelity shell emulation.
* **IPC Bridge:** Secure context bridging between the Electron main process and the React renderer via a typed `preload/index.js` API.
* **MCP Server:** `src/mcp/index.js` — compiled into a self-contained `resources/mcp.js` bundle for production installs.

---

## Security

Security is a first-class citizen in Glyph:
* **Local-Only:** All data, credentials, and settings remain on your local machine.
* **Zero Telemetry:** Glyph does not track your usage or send data to third parties.
* **AES-256 Encryption:** The server vault and exported configurations are encrypted using industry-standard AES-256.

---

## AI Agent Setup (MCP)

Glyph natively supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io), letting AI assistants interact with your infrastructure securely.

### In-App Setup (Recommended)

Open Glyph and expand the **"AI Agent Setup"** panel at the bottom of the home screen. It will automatically detect which AI clients are installed on your machine and show their current configuration status. Click **Auto-Install** next to any client to have Glyph write the correct config in one step.

**Supported clients (auto-install):**
| Client | Config location |
|---|---|
| Antigravity IDE | `~/.gemini/config/mcp_config.json` |
| Claude Desktop | `%APPDATA%\Claude\claude_desktop_config.json` |
| Cursor | `~/.cursor/mcp.json` |
| VS Code | `%APPDATA%\Code\User\settings.json` (under `mcp.servers`) |

For any other MCP-compatible tool, use the **Custom / Manual Setup** drawer to browse to its config file, pick the key format, preview the JSON, and write it in one click.

### Manual Config Snippet

If you prefer to configure manually, add the following entry to your AI client's MCP config:

```json
"glyph_mcp": {
  "command": "node",
  "args": ["/path/to/Glyph/resources/mcp.js"],
  "env": { "NODE_ENV": "production" }
}
```

> **Dev mode:** use `src/mcp/index.js` instead of `resources/mcp.js`.

### Available MCP Tools

| Tool | Description |
|---|---|
| `glyph_list_servers` | Returns all saved servers |
| `glyph_connect_server` | Opens a connection to a server in the Glyph UI |
| `glyph_execute_command` | Executes a shell command and returns the output |
| `glyph_read_file` / `glyph_write_file` | Read and write files via SFTP |
| `glyph_start_tunnel` / `glyph_stop_tunnel` | Start and stop port-forwarding tunnels |
| `glyph_list_secrets` / `glyph_add_secret` | Manage vault secrets |
| `glyph_list_containers` | View Docker containers on the server |
| `glyph_list_commands` / `glyph_add_command` / `glyph_remove_command` | Manage command snippets |

---

## Development

Glyph uses `electron-vite` for rapid development.

* `src/main` — Electron backend (SSH, Vault, API, Updater, MCP IPC handlers).
* `src/renderer` — React frontend application.
* `src/preload` — Typed IPC bridge exposed as `window.api`.
* `src/mcp` — Standalone MCP server (compiled to `resources/mcp.js` at build time via `scripts/bundle-mcp.cjs`).

Contributions, pull requests, and bug reports are highly welcome.
