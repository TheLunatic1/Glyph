## v2.6.0 - Agentic AI Integration via MCP

This release transforms Glyph into an intelligent SSH manager by integrating the Model Context Protocol (MCP), allowing AI agents to seamlessly interact with your secure servers. It also brings important bug fixes to the terminal and secrets vault.

### New Features & Fixes

- **Terminal Bug Fixes:** Fixed a critical issue where the terminal would not focus correctly on initial load, which interfered with normal copy and pasting.
- **Multi-Tab Secrets Injection:** Fixed an issue where injecting secrets from the vault into the terminal would fail if multiple terminal tabs were open.
- **Live Secret Syncing:** The Secrets UI now instantly updates when a new secret is added via an MCP agent.
- **Model Context Protocol (MCP) Integration:** Glyph now hosts an MCP server that bridges AI agents with your secure vault.
- **Local Authenticated API:** A secure, token-based local HTTP server runs inside the Glyph app, proxying requests from the MCP server to your active SSH sessions.
- **Visual AI Workspace:** When an AI agent executes a command, Glyph automatically opens an exclusive "AI Agent" visual terminal tab. Watch agents execute complex tasks on your servers in real-time.
- **Full Agentic Capabilities:** Agents can list your saved servers, connect to them, execute commands, read/write files via SFTP, list/add secrets to the vault, manage command snippets, tunnels, and Docker containers.
