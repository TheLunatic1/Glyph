/**
 * Bundles src/mcp/index.js into a single CJS file (resources/mcp.js)
 * with all @modelcontextprotocol/sdk deps inlined.
 * Run via: node scripts/bundle-mcp.cjs
 * Called automatically during `npm run build`.
 */

const esbuild = require('esbuild');
const path = require('path');
const fs = require('fs');

const outDir = path.resolve(__dirname, '..', 'resources');
if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true });

// Pre-process: replace import.meta.url with a CJS-compatible shim variable
// so the bundled output works when spawned as a plain Node.js child process.
const srcPath = path.resolve(__dirname, '..', 'src', 'mcp', 'index.js');
const originalSrc = fs.readFileSync(srcPath, 'utf8');
const patchedSrc = originalSrc.replace(/import\.meta\.url/g, '__importMetaUrl');

const tmpPath = path.join(outDir, '_mcp_tmp.js');
fs.writeFileSync(tmpPath, patchedSrc);

esbuild.buildSync({
  entryPoints: [tmpPath],
  bundle: true,
  platform: 'node',
  format: 'cjs',
  target: 'node18',
  outfile: path.join(outDir, 'mcp.js'),
  // Keep Node built-ins external — they're always available
  external: ['fs', 'os', 'path', 'child_process', 'url', 'net', 'stream',
             'events', 'buffer', 'util', 'crypto', 'http', 'https', 'zlib',
             'tls', 'readline', 'node:*'],
  minify: false,
  // Inject the CJS equivalent at the very top of the bundle
  banner: {
    js: [
      '#!/usr/bin/env node',
      '/* Glyph MCP Server — bundled, standalone build */',
      'const __importMetaUrl = require("url").pathToFileURL(__filename).href;',
    ].join('\n'),
  },
});

// Clean up temp file
fs.unlinkSync(tmpPath);

console.log('✅  MCP server bundled → resources/mcp.js');
