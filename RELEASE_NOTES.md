## v2.7.3 — MCP Auto-Launch, System Tray, Settings Modal & UI Polish

### What's New & Improvements

- **System Tray Integration & Quick Connect:** Glyph now integrates directly into the system tray with a dynamic menu that lists all your saved servers for instantaneous 1-click connection launches into dedicated windows, alongside options to show or hide the main app.
- **MCP Auto-Launch & Path Resolution:** Fixed executable path resolution in the bundled MCP server (`resources/mcp.js`), ensuring AI agents (Antigravity, Claude, Cursor, VS Code) can automatically launch and communicate with `Glyph.exe` even when Glyph isn't already open.
- **Modular Settings Modal:** Moved the AI Agent (MCP) configuration into a dedicated Settings modal accessible via the gear icon on the top header, keeping the home dashboard clean and focused.
- **Custom TitleBar & Window Dragging:** Built a custom title bar with window drag support, dynamic server title display, and left-aligned author attribution.
- **Server Window Connection Lifecycle & Retry:** Fixed an issue where canceling or encountering an error during connection in dedicated server windows left an inactive dashboard. Canceling or closing now properly closes the dedicated window via IPC, and a **Retry** button was added on connection failures.
- **Default `root` Username:** The Add Server form now consistently pre-fills `username: "root"` and `port: 22` across all open, cancel, and save actions.
- **Polished Splash Screen:** Restored the animated Glyph splash screen during app launch and reloads with a smooth breathing glow and fade transition.
- **UI & Layout Enhancements:** Refined header layout with version badges, cleaner footer styling, and improved scrollbar theming.
