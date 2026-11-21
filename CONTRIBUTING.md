# Contributing to Glyph

Thank you for contributing to **Glyph**!

## Development Prerequisites

- **Rust stable (`1.85+`, 2021 edition)** via [rustup.rs](https://rustup.rs)
- **Linux only (Debian / Ubuntu):**
  ```bash
  sudo apt-get update && sudo apt-get install -y \
    libx11-dev libxkbcommon-dev libwayland-dev libgtk-3-dev libxdo-dev \
    libasound2-dev libdbus-1-dev libxcursor-dev libxi-dev libxinerama-dev \
    libxrandr-dev libvulkan-dev libfontconfig1-dev pkg-config
  ```

## Local Setup & Workflow

1. Clone the repository:
   ```bash
   git clone https://github.com/TheLunatic1/Glyph.git
   cd Glyph
   ```
2. Run lint checks (CI enforces zero warnings):
   ```bash
   cargo clippy --workspace --all-targets -- -D warnings
   ```
3. Run the full unit and loopback E2E test suite:
   ```bash
   cargo test --workspace
   ```
4. Launch the desktop app from source:
   ```bash
   cargo run -p glyph-desktop
   ```

## Guidelines

- Keep all crates **100% pure Rust without CMake, NASM, OpenSSL, or `aws-lc-sys`**. CI verifies `cargo tree` on every PR.
- Add or update unit/E2E tests in the matching crate (`glyph-core`, `glyph-terminal`, `glyph-vnc`, `glyph-api`, `glyph-mcp`) for any new behavior or MCP route.
- Update [`README.md`](README.md) and [`docs/MCP.md`](docs/MCP.md) when adding MCP tools or user-facing configuration changes.
