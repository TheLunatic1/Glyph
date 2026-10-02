## v2.7.5 — Container Search, Single-Instance Lock & MCP Stability

### What's New & Improvements

- **Docker Container Real-time Search:** Added an instant search and filter bar to the Docker Containers view, allowing you to instantly search across container Names, Images, IDs, States, and Statuses with live match count badges and a 1-click clear control.
- **Single-Instance Enforcement & Tray Polish:** Implemented strict single-instance locking (`app.requestSingleInstanceLock()`). Opening Glyph while it is minimized in the tray now automatically restores and focuses the active window, eliminating duplicate system tray icons and preventing multiple background instances.
- **Persistent AI Agent (MCP) Stability:** Hardened the Local MCP API server (port `15354`) with dedicated port collision protection and token persistence, ensuring AI agents (Claude Desktop, Claude Code, Antigravity, Cursor, VS Code) maintain uninterrupted connectivity without requiring manual app restarts.
- **Claude Desktop MSIX & Claude Code Support:** Full auto-discovery and 1-click auto-install for both Windows Store (MSIX) Claude Desktop installations and the Claude Code CLI terminal agent.
- **Selective Server Export & Import:** Selective checkboxes with Select/Deselect All controls for exporting and importing `.glyph` encrypted server vaults.
- **Polished Glassmorphic UI:** Smooth breathing splash screen, custom draggable title bar with left-aligned author attribution, and consistent default `root` server setup credentials.
