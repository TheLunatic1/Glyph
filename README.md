# Glyph

Glyph is a modern, high-performance SSH client and server management application built with Electron and React. Designed for power users, developers, and system administrators, Glyph combines a beautiful user interface with powerful networking and automation features.

---

## Table of Contents

1. [Features](#features)
2. [Installation](#installation)
3. [Architecture](#architecture)
4. [Security](#security)
5. [Model Context Protocol (MCP)](#model-context-protocol-mcp)
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
* **Built-in ZeroTier Integration:** Connect directly to ZeroTier virtual networks from within the application, bypassing the need for complex VPN configurations.
* **Visual SFTP Client:** Transfer, read, and edit files on remote servers using the integrated file browser.

### Automation & AI
* **Command Snippets:** Save and execute frequently used shell commands.
* **Secret Injection:** Store environment variables and secrets, securely injecting them into your terminal when needed.
* **Agentic AI Support (MCP):** Connect AI agents to your servers via the Model Context Protocol to automate infrastructure tasks visually.

---

## Installation

### Prerequisites
* Node.js v18 or newer
* npm or yarn

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

To package the application for your operating system:

```bash
npm run build
npm run pack
```

---

## Architecture

Glyph utilizes a modern stack to ensure high performance and maintainability:

* **Frontend:** React 18, Tailwind CSS, Lucide React (Icons).
* **Backend (Main Process):** Electron, Node.js, `ssh2` for core networking.
* **Terminal Emulator:** xterm.js for high-fidelity shell emulation.
* **IPC (Inter-Process Communication):** Secure context bridging between the Electron main process and the React renderer.

---

## Security

Security is a first-class citizen in Glyph:
* **Local-Only:** All data, credentials, and settings remain on your local machine.
* **Zero Telemetry:** Glyph does not track your usage or send data to third parties.
* **AES-256 Encryption:** The server vault and exported configurations are encrypted using industry-standard AES-256.

---

## Model Context Protocol (MCP)

Glyph natively supports the Model Context Protocol (MCP), allowing AI agents to interact with your infrastructure securely.

### Starting the MCP Server

The MCP server runs via standard STDIO and can be attached to any compatible AI assistant.

```bash
node src/mcp/index.js
```

### Available Tools
* `glyph_list_servers`: Returns a list of all saved servers.
* `glyph_connect_server`: Opens a connection to a specific server in the Glyph application UI.
* `glyph_execute_command`: Visually types and executes a shell command on a connected server, returning the raw output to the agent.
* `glyph_read_file` / `glyph_write_file`: Read and write files to the server via SFTP.
* `glyph_start_tunnel` / `glyph_stop_tunnel`: Start and stop local port forwarding tunnels.
* `glyph_list_secrets` / `glyph_add_secret`: Manage secure secrets in the vault.
* `glyph_list_containers`: View Docker containers running on the server.
* `glyph_list_commands` / `glyph_add_command` / `glyph_remove_command`: Manage quick commands (snippets).

---

## Development

Glyph uses `electron-vite` for rapid development. 

* The `src/main` directory contains the Electron backend logic (SSH Manager, Vault, API, Update mechanisms).
* The `src/renderer` directory contains the React frontend application.
* The `src/preload` directory contains the secure IPC bridge.
* The `src/mcp` directory contains the standalone Model Context Protocol server.

Contributions, pull requests, and bug reports are highly welcome.
