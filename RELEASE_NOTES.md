## v2.7.0 — In-App AI Agent Setup

This release makes connecting AI assistants to Glyph dead simple. No more manually editing config files or hunting for the right JSON path — the entire MCP setup experience is now built directly into the home screen.

### What's New

#### In-App AI Agent Setup Panel
A collapsible **"AI Agent Setup"** panel now lives on the home page, right below your server list. Click it to instantly see the connection status for every supported AI client and configure them without leaving Glyph.

- **Per-client status indicators** for Antigravity IDE, Claude Desktop, Cursor, and VS Code — each showing whether it's configured, not yet set up, or not installed on this machine.
- **One-click Auto-Install** — Glyph writes the correct MCP config entry directly into each client's config file. Supports standard `mcpServers` format as well as VS Code's `mcp.servers` layout inside `settings.json`.
- **Manual snippet view** — expand any client card to copy a ready-to-paste JSON snippet for manual configuration.
- **Custom / Manual Setup drawer** — connect *any* MCP-compatible AI tool (open-source agents, custom scripts, etc.) by browsing to its config file and choosing the right key format. Supports dot-notation keys for nested configs and shows a live JSON preview before writing.

#### Bundled, Self-Contained MCP Server
Previously the MCP server ran from the source directory, which broke for installed (non-dev) users. The MCP server is now compiled into a single standalone bundle (`resources/mcp.js`) with all dependencies inlined — no `node_modules` required at runtime.

- In **dev mode**: runs directly from `src/mcp/index.js` as before.
- In **production installs**: runs from `resources/mcp.js` placed alongside the app via `extraResources`, correctly resolved via `process.resourcesPath`.
- The build pipeline (`npm run build` / `npm run dist`) automatically bundles the MCP server before packaging.

### Changes

- Removed the separate **Agent** sidebar tab — the setup panel is now on the home page where it's always accessible.
- MCP script path is no longer exposed as a raw file path in the UI (it's internal to the bundle now).
- `scripts/bundle-mcp.cjs` added as the MCP bundler (uses esbuild, already a transitive dev dependency via Vite).

---

## v2.6.0 — Agentic AI Integration via MCP

This release transforms Glyph into an intelligent SSH manager by integrating the Model Context Protocol (MCP), allowing AI agents to seamlessly interact with your secure servers. It also brings important bug fixes to the terminal and secrets vault.

### New Features & Fixes

- **Terminal Bug Fixes:** Fixed a critical issue where the terminal would not focus correctly on initial load, which interfered with normal copy and pasting.
- **Multi-Tab Secrets Injection:** Fixed an issue where injecting secrets from the vault into the terminal would fail if multiple terminal tabs were open.
- **Live Secret Syncing:** The Secrets UI now instantly updates when a new secret is added via an MCP agent.
- **Model Context Protocol (MCP) Integration:** Glyph now hosts an MCP server that bridges AI agents with your secure vault.
- **Local Authenticated API:** A secure, token-based local HTTP server runs inside the Glyph app, proxying requests from the MCP server to your active SSH sessions.
- **Visual AI Workspace:** When an AI agent executes a command, Glyph automatically opens an exclusive "AI Agent" visual terminal tab. Watch agents execute complex tasks on your servers in real-time.
- **Full Agentic Capabilities:** Agents can list your saved servers, connect to them, execute commands, read/write files via SFTP, list/add secrets to the vault, manage command snippets, tunnels, and Docker containers.
