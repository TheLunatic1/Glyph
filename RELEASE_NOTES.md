## v2.7.4 — Claude Desktop MSIX Fix, Claude Code CLI Support & Selective Server Export

### What's New & Bug Fixes

- **Claude Desktop (Windows Store / MSIX) Fix:** Fixed an issue where Claude Desktop installed via Microsoft Store / WindowsApp MSIX package was undetected and reported as "Not Installed". Glyph now scans `%LOCALAPPDATA%\Packages\Claude_*\LocalCache\Roaming\Claude\` alongside standard `%APPDATA%\Claude` paths and syncs MCP configs automatically.
- **Claude Code (CLI) Integration:** Added first-class support for Anthropic's Claude Code terminal agent (`claude`) in the Settings modal with 1-click auto-install directly to `~/.claude.json`.
- **Selective Server Export & Import:** Users can now pick and choose exactly which servers to export into encrypted backup files with Select All / Deselect All controls, live selected counters, and server previews, alongside selective server selection when decrypting and importing backups.
- **System Tray & Window Lifecycle Fixes:** Resized the system tray icon to 16x16 with multiple fallback paths to prevent invisible tray icons on Windows, and ensured closing the window hides it cleanly to the tray without terminating background SSH sessions.
- **Default `root` Username:** Pre-fills `username: "root"` and `port: 22` consistently across all Add Server form states and resets.
- **Modular Settings Modal:** Clean, dedicated modal for configuring AI Agents (MCP) from the top header.
- **Custom TitleBar & Window Dragging:** Built a custom title bar with window drag support, dynamic server title display, and left-aligned author attribution.
- **Server Window Connection Lifecycle & Retry:** Proper IPC window close on connection cancel/error with a **Retry** option.
- **Polished Splash Screen:** Smooth breathing glow and fade-out transition on app launch and reloads.
