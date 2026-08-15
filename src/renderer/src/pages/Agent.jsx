import React, { useState, useEffect, useCallback } from 'react';
import {
  Bot, CheckCircle2, XCircle, AlertCircle, Copy, Check,
  RefreshCw, ChevronDown, ChevronRight, Zap, FolderOpen,
  Settings, Info, Wrench
} from 'lucide-react';

// ── Client metadata ────────────────────────────────────────────────────────────
const CLIENTS = {
  antigravity: {
    name: 'Antigravity IDE',
    description: 'Google DeepMind AI coding assistant',
    icon: '⚡',
    gradient: 'from-blue-500/20 to-purple-600/20',
    border: 'border-blue-500/30',
    accent: 'text-blue-400',
    snippetKey: 'mcpServers',
  },
  claude: {
    name: 'Claude Desktop',
    description: 'Anthropic Claude AI assistant',
    icon: '🤖',
    gradient: 'from-orange-500/20 to-amber-500/20',
    border: 'border-orange-500/30',
    accent: 'text-orange-400',
    snippetKey: 'mcpServers',
  },
  cursor: {
    name: 'Cursor',
    description: 'AI-powered code editor',
    icon: '◎',
    gradient: 'from-violet-500/20 to-blue-500/20',
    border: 'border-violet-500/30',
    accent: 'text-violet-400',
    snippetKey: 'mcpServers',
  },
  vscode: {
    name: 'VS Code',
    description: 'Visual Studio Code + GitHub Copilot',
    icon: '⬡',
    gradient: 'from-sky-500/20 to-blue-600/20',
    border: 'border-sky-500/30',
    accent: 'text-sky-400',
    snippetKey: 'mcp.servers',
  },
};

// ── Status badge ───────────────────────────────────────────────────────────────
function StatusBadge({ status }) {
  if (status === 'configured') {
    return (
      <span className="flex items-center gap-1.5 text-xs font-semibold text-emerald-400 bg-emerald-400/10 border border-emerald-400/20 px-2.5 py-0.5 rounded-full">
        <CheckCircle2 size={11} /> Configured
      </span>
    );
  }
  if (status === 'not-configured') {
    return (
      <span className="flex items-center gap-1.5 text-xs font-semibold text-amber-400 bg-amber-400/10 border border-amber-400/20 px-2.5 py-0.5 rounded-full">
        <AlertCircle size={11} /> Not Configured
      </span>
    );
  }
  return (
    <span className="flex items-center gap-1.5 text-xs font-semibold text-gray-500 bg-dark-700 border border-dark-700 px-2.5 py-0.5 rounded-full">
      <XCircle size={11} /> Not Installed
    </span>
  );
}

// ── Copy-able code block ───────────────────────────────────────────────────────
function SnippetBlock({ code }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };
  return (
    <div className="relative group">
      <pre className="bg-dark-900 border border-dark-700 rounded-lg p-3 text-xs text-gray-300 font-mono overflow-x-auto leading-relaxed whitespace-pre-wrap break-all">
        {code}
      </pre>
      <button
        onClick={handleCopy}
        className="absolute top-2 right-2 p-1.5 rounded-md bg-dark-700 hover:bg-dark-600 text-gray-400 hover:text-gray-200 transition-all"
        title="Copy to clipboard"
      >
        {copied ? <Check size={13} className="text-emerald-400" /> : <Copy size={13} />}
      </button>
    </div>
  );
}

// ── Per-client card ────────────────────────────────────────────────────────────
function ClientCard({ clientId, info, mcpScriptPath, onInstall }) {
  const meta = CLIENTS[clientId];
  const [expanded, setExpanded] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [installResult, setInstallResult] = useState(null);

  const handleInstall = async () => {
    setInstalling(true);
    setInstallResult(null);
    try {
      const result = await window.api.mcpAutoInstall(clientId);
      setInstallResult(result);
      if (result.success) onInstall();
    } catch (e) {
      setInstallResult({ success: false, error: e.message });
    } finally {
      setInstalling(false);
    }
  };

  const scriptPathFwd = mcpScriptPath ? mcpScriptPath.replace(/\\/g, '/') : '...path/to/mcp.js';

  let snippet;
  if (clientId === 'vscode') {
    snippet = `// Add to your settings.json under "mcp.servers":\n"glyph_mcp": {\n  "type": "stdio",\n  "command": "node",\n  "args": ["${scriptPathFwd}"],\n  "env": { "NODE_ENV": "production" }\n}`;
  } else {
    snippet = `// Add to your mcpServers config:\n"glyph_mcp": {\n  "command": "node",\n  "args": ["${scriptPathFwd}"],\n  "env": { "NODE_ENV": "production" }\n}`;
  }

  const isInstalled = info.status !== 'not-installed';
  const isConfigured = info.status === 'configured';

  return (
    <div className={`rounded-xl border bg-gradient-to-br ${meta.gradient} ${meta.border} transition-all duration-200 overflow-hidden`}>
      <div className="p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-center gap-3 min-w-0">
            <div className={`w-10 h-10 rounded-lg bg-gradient-to-br ${meta.gradient} border ${meta.border} flex items-center justify-center text-lg flex-shrink-0`}>
              {meta.icon}
            </div>
            <div className="min-w-0">
              <div className="flex items-center gap-2 flex-wrap">
                <span className={`font-semibold text-sm ${meta.accent}`}>{meta.name}</span>
                <StatusBadge status={info.status} />
              </div>
              <p className="text-xs text-gray-500 mt-0.5">{meta.description}</p>
            </div>
          </div>
          <div className="flex items-center gap-2 flex-shrink-0">
            {isInstalled && !isConfigured && (
              <button
                onClick={handleInstall}
                disabled={installing}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-brand-500/20 hover:bg-brand-500/30 border border-brand-500/40 text-brand-400 hover:text-brand-300 rounded-lg text-xs font-semibold transition-all disabled:opacity-50"
              >
                {installing ? <RefreshCw size={12} className="animate-spin" /> : <Zap size={12} />}
                {installing ? 'Installing…' : 'Auto-Install'}
              </button>
            )}
            {isConfigured && (
              <button
                onClick={handleInstall}
                disabled={installing}
                title="Re-apply / update the Glyph entry"
                className="flex items-center gap-1.5 px-2.5 py-1.5 bg-dark-700/60 hover:bg-dark-700 border border-dark-700 text-gray-400 hover:text-gray-200 rounded-lg text-xs transition-all disabled:opacity-50"
              >
                <RefreshCw size={12} className={installing ? 'animate-spin' : ''} />
              </button>
            )}
            <button
              onClick={() => setExpanded(v => !v)}
              className="p-1.5 rounded-lg hover:bg-dark-700/60 text-gray-500 hover:text-gray-300 transition-colors"
            >
              {expanded ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
            </button>
          </div>
        </div>

        {installResult && (
          <div className={`mt-3 text-xs px-3 py-2 rounded-lg font-medium ${
            installResult.success
              ? 'bg-emerald-500/10 border border-emerald-500/20 text-emerald-400'
              : 'bg-red-500/10 border border-red-500/20 text-red-400'
          }`}>
            {installResult.success ? `✓ Written to: ${installResult.path}` : `✗ ${installResult.error}`}
          </div>
        )}
      </div>

      {expanded && (
        <div className="border-t border-white/5 px-4 pb-4 pt-3 space-y-3">
          <div>
            <p className="text-[10px] font-semibold text-gray-500 uppercase tracking-wider mb-1.5">Config File Path</p>
            <p className="text-xs text-gray-400 font-mono bg-dark-900/60 px-2.5 py-2 rounded-lg border border-dark-700 break-all">
              {info.configPath || 'Unknown'}
            </p>
          </div>
          <div>
            <p className="text-[10px] font-semibold text-gray-500 uppercase tracking-wider mb-1.5">Manual Config Snippet</p>
            <SnippetBlock code={snippet} />
          </div>
        </div>
      )}
    </div>
  );
}

// ── Manual / Custom client setup section ──────────────────────────────────────
const FORMAT_OPTIONS = [
  { value: 'mcpServers', label: 'mcpServers  — Standard (Claude, Cursor, Antigravity…)' },
  { value: 'vscode', label: 'mcp.servers  — VS Code / GitHub Copilot' },
  { value: 'custom', label: 'Custom key   — Any other tool' },
];

function ManualSetup({ mcpScriptPath }) {
  const [open, setOpen] = useState(false);
  const [configPath, setConfigPath] = useState('');
  const [format, setFormat] = useState('mcpServers');
  const [customKey, setCustomKey] = useState('');
  const [writing, setWriting] = useState(false);
  const [result, setResult] = useState(null);

  const scriptPathFwd = mcpScriptPath ? mcpScriptPath.replace(/\\/g, '/') : '...path...';

  // Build a live preview of what will be written
  const buildPreview = () => {
    const entry = {
      command: 'node',
      args: [scriptPathFwd],
      env: { NODE_ENV: 'production' }
    };
    if (format === 'vscode') {
      return JSON.stringify({ mcp: { servers: { glyph_mcp: { type: 'stdio', ...entry } } } }, null, 2);
    } else if (format === 'custom' && customKey) {
      const key = customKey || 'your.key';
      const parts = key.split('.');
      let obj = { glyph_mcp: entry };
      for (let i = parts.length - 1; i >= 0; i--) obj = { [parts[i]]: obj };
      return JSON.stringify(obj, null, 2);
    }
    return JSON.stringify({ mcpServers: { glyph_mcp: entry } }, null, 2);
  };

  const handleBrowse = async () => {
    const p = await window.api.mcpOpenFileDialog();
    if (p) { setConfigPath(p); setResult(null); }
  };

  const handleWrite = async () => {
    if (!configPath) return;
    setWriting(true);
    setResult(null);
    try {
      const res = await window.api.mcpWriteCustom({ configPath, format, customKey });
      setResult(res);
    } catch (e) {
      setResult({ success: false, error: e.message });
    } finally {
      setWriting(false);
    }
  };

  return (
    <div className="rounded-xl border border-dark-700 bg-dark-800 overflow-hidden">
      {/* Header toggle */}
      <button
        onClick={() => setOpen(v => !v)}
        className="w-full flex items-center justify-between px-4 py-3 hover:bg-dark-700/40 transition-colors"
      >
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-dark-700 border border-dark-700 flex items-center justify-center">
            <Wrench size={15} className="text-gray-400" />
          </div>
          <div className="text-left">
            <p className="text-sm font-semibold text-gray-200">Custom / Manual Setup</p>
            <p className="text-xs text-gray-500">Connect any MCP-compatible tool not listed above</p>
          </div>
        </div>
        {open ? <ChevronDown size={15} className="text-gray-500" /> : <ChevronRight size={15} className="text-gray-500" />}
      </button>

      {open && (
        <div className="border-t border-dark-700 px-4 pb-4 pt-3 space-y-4">

          {/* Info callout */}
          <div className="flex gap-2 text-xs text-gray-400 bg-brand-500/5 border border-brand-500/20 rounded-lg px-3 py-2">
            <Info size={13} className="text-brand-400 flex-shrink-0 mt-0.5" />
            <span>
              Works with any tool that reads a JSON config file and supports MCP stdio servers —
              open-source AI agents, custom scripts, etc.
            </span>
          </div>

          {/* Config file path */}
          <div>
            <label className="text-[10px] font-semibold text-gray-500 uppercase tracking-wider block mb-1.5">
              Config File Path
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={configPath}
                onChange={e => { setConfigPath(e.target.value); setResult(null); }}
                placeholder="/path/to/your-tool-config.json"
                className="flex-1 bg-dark-900 border border-dark-700 rounded-lg px-3 py-2 text-xs text-gray-200 font-mono placeholder-gray-600 focus:outline-none focus:border-brand-500/60"
              />
              <button
                onClick={handleBrowse}
                className="flex items-center gap-1.5 px-3 py-2 bg-dark-700 hover:bg-dark-600 border border-dark-700 text-gray-400 hover:text-gray-200 rounded-lg text-xs transition-all"
                title="Browse for config file"
              >
                <FolderOpen size={13} /> Browse
              </button>
            </div>
          </div>

          {/* Config format */}
          <div>
            <label className="text-[10px] font-semibold text-gray-500 uppercase tracking-wider block mb-1.5">
              Config Format
            </label>
            <select
              value={format}
              onChange={e => setFormat(e.target.value)}
              className="w-full bg-dark-900 border border-dark-700 rounded-lg px-3 py-2 text-xs text-gray-200 focus:outline-none focus:border-brand-500/60 appearance-none"
            >
              {FORMAT_OPTIONS.map(o => (
                <option key={o.value} value={o.value}>{o.label}</option>
              ))}
            </select>

            {format === 'custom' && (
              <div className="mt-2">
                <input
                  type="text"
                  value={customKey}
                  onChange={e => setCustomKey(e.target.value)}
                  placeholder="e.g.  mcpServers  or  aiTool.servers"
                  className="w-full bg-dark-900 border border-dark-700 rounded-lg px-3 py-2 text-xs text-gray-200 font-mono placeholder-gray-600 focus:outline-none focus:border-brand-500/60"
                />
                <p className="text-[10px] text-gray-500 mt-1">Supports dot notation for nested keys</p>
              </div>
            )}
          </div>

          {/* Live preview */}
          {configPath && (
            <div>
              <p className="text-[10px] font-semibold text-gray-500 uppercase tracking-wider mb-1.5">Preview — what will be merged in</p>
              <SnippetBlock code={buildPreview()} />
            </div>
          )}

          {/* Write button + result */}
          <div className="flex items-center gap-3">
            <button
              onClick={handleWrite}
              disabled={!configPath || writing}
              className="flex items-center gap-2 px-4 py-2 bg-brand-500/20 hover:bg-brand-500/30 border border-brand-500/40 text-brand-400 hover:text-brand-300 rounded-lg text-xs font-semibold transition-all disabled:opacity-40"
            >
              {writing ? <RefreshCw size={12} className="animate-spin" /> : <Settings size={12} />}
              {writing ? 'Writing…' : 'Write to Config'}
            </button>
            {result && (
              <span className={`text-xs font-medium ${result.success ? 'text-emerald-400' : 'text-red-400'}`}>
                {result.success ? `✓ Written to: ${result.path}` : `✗ ${result.error}`}
              </span>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Main Agent page ────────────────────────────────────────────────────────────
export default function Agent() {
  const [mcpInfo, setMcpInfo] = useState(null);
  const [loading, setLoading] = useState(true);

  const loadInfo = useCallback(async () => {
    setLoading(true);
    try {
      const info = await window.api.mcpGetInfo();
      setMcpInfo(info);
    } catch (e) {
      console.error('Failed to load MCP info:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadInfo(); }, [loadInfo]);

  const configuredCount = mcpInfo
    ? Object.values(mcpInfo.clients).filter(c => c.status === 'configured').length
    : 0;

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Header */}
      <div className="flex-shrink-0 px-6 py-5 border-b border-dark-700 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg bg-brand-500/15 border border-brand-500/30 flex items-center justify-center">
            <Bot size={18} className="text-brand-400" />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-gray-100">AI Agent Integration</h2>
            <p className="text-xs text-gray-500 mt-0.5">Connect AI assistants to your servers via MCP</p>
          </div>
        </div>
        <button
          onClick={loadInfo}
          disabled={loading}
          className="flex items-center gap-1.5 px-3 py-1.5 text-xs text-gray-400 hover:text-gray-200 bg-dark-700 hover:bg-dark-600 rounded-lg border border-dark-700 transition-all"
        >
          <RefreshCw size={12} className={loading ? 'animate-spin' : ''} />
          Refresh
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-6 py-5 space-y-4">

        {/* Global server status */}
        <div className="flex items-center justify-between bg-dark-800 border border-dark-700 rounded-xl px-4 py-3">
          <div className="flex items-center gap-3">
            <span className="relative flex h-3 w-3">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-60"></span>
              <span className="relative inline-flex rounded-full h-3 w-3 bg-emerald-500"></span>
            </span>
            <div>
              <p className="text-xs font-semibold text-gray-200">Glyph MCP Server</p>
              <p className="text-[10px] text-gray-500">Running on port 15354 · STDIO transport · Standalone bundle</p>
            </div>
          </div>
          <div className="text-right">
            <p className="text-[10px] text-gray-500">Configured in</p>
            <p className="text-sm font-bold text-brand-400">{configuredCount} / {Object.keys(CLIENTS).length} clients</p>
          </div>
        </div>

        {/* How it works */}
        <div className="bg-brand-500/5 border border-brand-500/20 rounded-xl px-4 py-3 text-xs text-gray-400 leading-relaxed">
          <p className="font-semibold text-brand-300 mb-1">How it works</p>
          Glyph ships a self-contained MCP server (<span className="text-brand-300 font-mono">mcp.js</span>).
          Your AI assistant connects to it using the config below.
          Once set up, the agent can list servers, run commands, manage files via SFTP,
          control tunnels, and more — all through your AI chat.
        </div>

        {/* Per-client cards */}
        {loading ? (
          <div className="flex items-center justify-center py-12 text-gray-500">
            <RefreshCw size={18} className="animate-spin mr-2" />
            <span className="text-sm">Scanning for AI clients…</span>
          </div>
        ) : mcpInfo ? (
          <div className="space-y-3">
            <p className="text-[10px] font-semibold text-gray-500 uppercase tracking-wider">Supported Clients</p>
            {Object.entries(CLIENTS).map(([clientId]) => (
              <ClientCard
                key={clientId}
                clientId={clientId}
                info={mcpInfo.clients[clientId] || { status: 'not-installed', configPath: '' }}
                mcpScriptPath={mcpInfo.mcpScriptPath}
                onInstall={loadInfo}
              />
            ))}

            {/* Manual setup for any other tool */}
            <div className="pt-1">
              <p className="text-[10px] font-semibold text-gray-500 uppercase tracking-wider mb-2">Other Tools</p>
              <ManualSetup mcpScriptPath={mcpInfo.mcpScriptPath} />
            </div>
          </div>
        ) : (
          <div className="text-center py-12 text-gray-500 text-sm">Failed to load MCP information.</div>
        )}

        {/* MCP script path */}
        {mcpInfo?.mcpScriptPath && (
          <div className="bg-dark-800 border border-dark-700 rounded-xl px-4 py-3">
            <p className="text-[10px] font-semibold text-gray-500 uppercase tracking-wider mb-1.5">MCP Script Path</p>
            <p className="text-xs font-mono text-gray-400 break-all">{mcpInfo.mcpScriptPath}</p>
          </div>
        )}
      </div>
    </div>
  );
}
