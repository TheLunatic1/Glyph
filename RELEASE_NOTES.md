## v2.4.0 — The Ultimate Native & Docker Update

This massive update brings native-level features to your File Explorer, introduces full Docker & Secrets management, and supports running multiple Glyph instances simultaneously!

### New Features

- **SFTP Native Overhaul:**
  - **Multi-Select System:** Select multiple files using `Shift+Click` (range) or `Ctrl/Cmd+Click` (individual).
  - **Internal Drag & Drop:** Easily move files and folders around within your remote server simply by dragging them.
  - **Local to Remote Drop:** Recursive folder dropping allows you to drag entire nested folders from your local machine directly into the SFTP explorer.
  - **Keyboard Shortcuts:** Full support for `Ctrl+C`, `Ctrl+X`, `Ctrl+V`, and `Delete` right inside the file explorer for a true native OS feel.
  - **Transfer Progress Bars:** Beautiful sliding progress overlays for active uploads and downloads so you always know their status.
  - **Smart Context Menus:** Right-click on files or directly in empty space to create new items, paste, or refresh.
  - **Copy Path:** A new quick-access "Copy Path" button right in the breadcrumbs.
  - **Constrained Editor:** Built-in code editor now perfectly overlays only the file view so your sidebar remains usable.

- **Docker & Secrets Management:**
  - **Containers Dashboard:** View and manage your active Docker containers natively inside Glyph.
  - **Secrets Vault:** A dedicated encrypted vault for managing sensitive tokens, API keys, and environment variables on your servers.

- **Multi-Instance Support:** 
  - **Run Multiple Glyphs:** Need to manage multiple servers at the exact same time? You can now open multiple standalone instances of Glyph in separate windows!

- **Terminal Enhancements:**
  - **Multi-Terminal Setups:** Extended terminal management capabilities for running multiple concurrent terminal sessions smoothly.

### Bug Fixes & Improvements

- Greatly optimized background file fetching and memory usage during SFTP transfers.
- Fixed a layout issue where the internal editor would sometimes stretch across the entire application window.
- Overall stability and speed improvements across the SSH connection pool.
