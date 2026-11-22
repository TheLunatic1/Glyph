#!/usr/bin/env node
// Compatibility stdio proxy for legacy Glyph v2 (Electron) MCP configs
// pointing at `<install>/resources/mcp.js`. Forwards stdio straight to Pure Rust `glyph-mcp`.
const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");

const binName = process.platform === "win32" ? "glyph-mcp.exe" : "glyph-mcp";
const candidates = [
  path.resolve(__dirname, "..", binName),
  path.resolve(__dirname, "..", "MacOS", binName),
  path.resolve(__dirname, binName),
  binName,
];

const target = candidates.find((p) => p === binName || fs.existsSync(p));
const child = spawn(target, process.argv.slice(2), {
  stdio: "inherit",
  windowsHide: true,
});

child.on("error", (err) => {
  process.stderr.write(`Failed to launch ${target}: ${err.message}\n`);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 0);
});

for (const sig of ["SIGINT", "SIGTERM"]) {
  process.on(sig, () => {
    if (!child.killed) child.kill(sig);
  });
}
