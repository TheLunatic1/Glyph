import React, { useState, useCallback, useEffect } from 'react';
import {
  Bot, CheckCircle2, XCircle, AlertCircle, ChevronDown, ChevronRight,
  Zap, RefreshCw, Copy, Check, FolderOpen, Settings, Info, X,
  Smartphone, QrCode, Download, ExternalLink, ShieldCheck, Terminal, Layers, Cpu, Sparkles
} from 'lucide-react';

// ── Client definitions ────────────────────────────────────────────────────────
const CLIENTS = {
  antigravity: { name: 'Antigravity IDE',   icon: '⚡', accent: 'text-blue-400',   border: 'border-blue-500/30',   bg: 'bg-blue-500/10'   },
  claude:      { name: 'Claude Desktop',    icon: '🤖', accent: 'text-orange-400', border: 'border-orange-500/30', bg: 'bg-orange-500/10' },
  claudecode:  { name: 'Claude Code (CLI)', icon: '💻', accent: 'text-amber-400',  border: 'border-amber-500/30',  bg: 'bg-amber-500/10'  },
  cursor:      { name: 'Cursor',            icon: '◎',  accent: 'text-violet-400', border: 'border-violet-500/30', bg: 'bg-violet-500/10' },
  vscode:      { name: 'VS Code',           icon: '⬡',  accent: 'text-sky-400',    border: 'border-sky-500/30',    bg: 'bg-sky-500/10'    },
};

// ── Tiny helpers ──────────────────────────────────────────────────────────────
function StatusPip({ status }) {
  if (status === 'configured')     return <span className="flex items-center gap-1 text-[10px] font-semibold text-emerald-400"><CheckCircle2 size={10} /> Configured</span>;
  if (status === 'not-configured') return <span className="flex items-center gap-1 text-[10px] font-semibold text-amber-400"><AlertCircle size={10} /> Not set up</span>;
  return <span className="flex items-center gap-1 text-[10px] font-semibold text-gray-600"><XCircle size={10} /> Not installed</span>;
}

function CopyBtn({ text }) {
  const [copied, setCopied] = useState(false);
  const handle = () => { navigator.clipboard.writeText(text); setCopied(true); setTimeout(() => setCopied(false), 2000); };
  return (
    <button onClick={handle} className="absolute top-2 right-2 p-1 rounded bg-dark-700/80 hover:bg-dark-700 text-gray-500 hover:text-gray-300 transition-all">
      {copied ? <Check size={11} className="text-emerald-400" /> : <Copy size={11} />}
    </button>
  );
}

// ── Per-client card (own component so hooks are at top-level) ─────────────────
function ClientCard({ clientId, meta, info, result, installing, onInstall, mcpScriptPath }) {
  const [expanded, setExpanded] = useState(false);

  const isInstalled = info.status !== 'not-installed';
  const isConfigured = info.status === 'configured';

  const scriptPath = (mcpScriptPath || '...').replace(/\\/g, '/');
  const snippet = clientId === 'vscode'
    ? JSON.stringify({ mcp: { servers: { glyph_mcp: { type: 'stdio', command: 'node', args: [scriptPath], env: { NODE_ENV: 'production' } } } } }, null, 2)
    : JSON.stringify({ mcpServers: { glyph_mcp: { command: 'node', args: [scriptPath], env: { NODE_ENV: 'production' } } } }, null, 2);

  return (
    <div className={`rounded-xl border ${meta.border} ${meta.bg} overflow-hidden`}>
      <div className="px-3.5 py-3 flex items-center gap-3">
        <span className="text-base w-7 text-center flex-shrink-0">{meta.icon}</span>
        <div className="flex-1 min-w-0">
          <p className={`text-xs font-semibold ${meta.accent}`}>{meta.name}</p>
          <StatusPip status={info.status} />
        </div>
        <div className="flex items-center gap-1.5 flex-shrink-0">
          {isInstalled && !isConfigured && (
            <button
              onClick={() => onInstall(clientId)}
              disabled={installing === clientId}
              className="flex items-center gap-1 px-2.5 py-1 bg-brand-500/20 hover:bg-brand-500/30 border border-brand-500/40 text-brand-400 rounded-lg text-[11px] font-semibold transition-all disabled:opacity-50"
            >
              {installing === clientId ? <RefreshCw size={10} className="animate-spin" /> : <Zap size={10} />}
              Auto-Install
            </button>
          )}
          {isConfigured && (
            <button
              onClick={() => onInstall(clientId)}
              disabled={installing === clientId}
              title="Re-apply"
              className="p-1.5 rounded-lg bg-dark-700/60 hover:bg-dark-700 border border-dark-700 text-gray-500 hover:text-gray-300 transition-all"
            >
              <RefreshCw size={11} className={installing === clientId ? 'animate-spin' : ''} />
            </button>
          )}
          <button onClick={() => setExpanded(v => !v)} className="p-1.5 rounded-lg hover:bg-dark-700/40 text-gray-600 hover:text-gray-400 transition-colors">
            {expanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
          </button>
        </div>
      </div>

      {result && (
        <div className={`mx-3 mb-2 px-2.5 py-1.5 rounded-lg text-[10px] font-medium ${
          result.success ? 'bg-emerald-500/10 border border-emerald-500/20 text-emerald-400' : 'bg-red-500/10 border border-red-500/20 text-red-400'
        }`}>
          {result.success ? '✓ Config written' : `✗ ${result.error}`}
        </div>
      )}

      {expanded && (
        <div className="border-t border-white/5 px-3 pb-3 pt-2">
          <p className="text-[9px] font-semibold text-gray-600 uppercase tracking-wider mb-1">Manual snippet</p>
          <div className="relative">
            <pre className="bg-dark-900 border border-dark-700 rounded-lg p-2.5 text-[10px] text-gray-400 font-mono overflow-x-auto leading-relaxed whitespace-pre-wrap break-all">
              {snippet}
            </pre>
            <CopyBtn text={snippet} />
          </div>
        </div>
      )}
    </div>
  );
}

// ── Manual setup sub-panel ────────────────────────────────────────────────────
const FORMAT_OPTIONS = [
  { value: 'mcpServers', label: 'mcpServers — Standard (Claude, Cursor…)' },
  { value: 'vscode',     label: 'mcp.servers — VS Code / GitHub Copilot' },
  { value: 'custom',     label: 'Custom key — Any other tool' },
];

function ManualSetupPanel({ mcpScriptPath }) {
  const [open, setOpen] = useState(false);
  const [configPath, setConfigPath] = useState('');
  const [format, setFormat] = useState('mcpServers');
  const [customKey, setCustomKey] = useState('');
  const [writing, setWriting] = useState(false);
  const [result, setResult] = useState(null);

  const scriptPath = (mcpScriptPath || '...').replace(/\\/g, '/');

  const buildPreview = () => {
    const entry = { command: 'node', args: [scriptPath], env: { NODE_ENV: 'production' } };
    if (format === 'vscode') return JSON.stringify({ mcp: { servers: { glyph_mcp: { type: 'stdio', ...entry } } } }, null, 2);
    if (format === 'custom' && customKey) {
      const parts = customKey.split('.');
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
    setWriting(true); setResult(null);
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
    <div className="rounded-xl border border-dark-700 bg-dark-800/60 overflow-hidden">
      <button onClick={() => setOpen(v => !v)} className="w-full flex items-center justify-between px-4 py-3 hover:bg-dark-700/30 transition-colors">
        <div className="flex items-center gap-2.5">
          <Settings size={13} className="text-gray-500" />
          <span className="text-xs font-semibold text-gray-300">Custom / Manual Setup</span>
          <span className="text-[10px] text-gray-600">— Any other MCP-compatible tool</span>
        </div>
        {open ? <ChevronDown size={13} className="text-gray-600" /> : <ChevronRight size={13} className="text-gray-600" />}
      </button>

      {open && (
        <div className="border-t border-dark-700 px-4 pb-4 pt-3 space-y-3">
          <div className="flex gap-1.5 text-[10px] text-gray-400 bg-brand-500/5 border border-brand-500/20 rounded-lg px-2.5 py-2">
            <Info size={11} className="text-brand-400 flex-shrink-0 mt-0.5" />
            Works with any tool that reads a JSON config and supports MCP stdio servers — open-source agents, custom scripts, etc.
          </div>

          <div className="flex gap-2">
            <input
              type="text" value={configPath}
              onChange={e => { setConfigPath(e.target.value); setResult(null); }}
              placeholder="Path to config file…"
              className="flex-1 bg-dark-900 border border-dark-700 rounded-lg px-3 py-1.5 text-xs text-gray-200 font-mono placeholder-gray-600 focus:outline-none focus:border-brand-500/60"
            />
            <button onClick={handleBrowse} className="flex items-center gap-1 px-2.5 py-1.5 bg-dark-700 hover:bg-dark-600 border border-dark-700 text-gray-400 hover:text-gray-200 rounded-lg text-xs transition-all">
              <FolderOpen size={12} /> Browse
            </button>
          </div>

          <div className="flex gap-2">
            <select value={format} onChange={e => setFormat(e.target.value)} className="flex-1 bg-dark-900 border border-dark-700 rounded-lg px-3 py-1.5 text-xs text-gray-200 focus:outline-none focus:border-brand-500/60">
              {FORMAT_OPTIONS.map(o => <option key={o.value} value={o.value}>{o.label}</option>)}
            </select>
            {format === 'custom' && (
              <input type="text" value={customKey} onChange={e => setCustomKey(e.target.value)} placeholder="e.g. mcpServers or ai.servers"
                className="flex-1 bg-dark-900 border border-dark-700 rounded-lg px-3 py-1.5 text-xs text-gray-200 font-mono placeholder-gray-600 focus:outline-none focus:border-brand-500/60"
              />
            )}
          </div>

          {configPath && (
            <div className="relative">
              <p className="text-[9px] font-semibold text-gray-600 uppercase tracking-wider mb-1">Preview — will be merged in:</p>
              <pre className="bg-dark-900 border border-dark-700 rounded-lg p-2.5 text-[10px] text-gray-400 font-mono overflow-x-auto whitespace-pre-wrap break-all leading-relaxed">
                {buildPreview()}
              </pre>
              <CopyBtn text={buildPreview()} />
            </div>
          )}

          <div className="flex items-center gap-3">
            <button onClick={handleWrite} disabled={!configPath || writing}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-brand-500/20 hover:bg-brand-500/30 border border-brand-500/40 text-brand-400 rounded-lg text-xs font-semibold transition-all disabled:opacity-40"
            >
              {writing ? <RefreshCw size={11} className="animate-spin" /> : <Settings size={11} />}
              Write to Config
            </button>
            {result && (
              <span className={`text-[10px] font-medium ${result.success ? 'text-emerald-400' : 'text-red-400'}`}>
                {result.success ? `✓ Written` : `✗ ${result.error}`}
              </span>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Main Modal ────────────────────────────────────────────────────
export default function SettingsModal({ onClose, onOpenMobile }) {
  const [activeTab, setActiveTab] = useState('mcp');
  const [mcpInfo, setMcpInfo] = useState(null);
  const [loading, setLoading] = useState(true);
  const [installing, setInstalling] = useState(null);
  const [installResults, setInstallResults] = useState({});

  const loadInfo = useCallback(async () => {
    setLoading(true);
    try {
      const info = await window.api.mcpGetInfo();
      setMcpInfo(info);
    } catch (e) {
      console.error('MCP info load failed:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadInfo();
  }, [loadInfo]);

  const handleAutoInstall = async (clientId) => {
    setInstalling(clientId);
    setInstallResults(r => ({ ...r, [clientId]: null }));
    try {
      const res = await window.api.mcpAutoInstall(clientId);
      setInstallResults(r => ({ ...r, [clientId]: res }));
      if (res.success) loadInfo();
    } catch (e) {
      setInstallResults(r => ({ ...r, [clientId]: { success: false, error: e.message } }));
    } finally {
      setInstalling(null);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
      <div className="bg-dark-900 border border-dark-700 w-full max-w-4xl max-h-[85vh] h-full rounded-2xl shadow-2xl flex flex-col overflow-hidden">
        
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-dark-800 bg-dark-800/30 shrink-0">
          <div className="flex items-center gap-3">
            <Settings size={18} className="text-gray-400" />
            <h2 className="text-lg font-semibold text-gray-200">Settings</h2>
          </div>
          <button onClick={onClose} className="p-1.5 rounded-md hover:bg-dark-700 text-gray-400 hover:text-gray-200 transition-colors">
            <X size={18} />
          </button>
        </div>
        
        {/* Body */}
        <div className="flex flex-1 overflow-hidden">
          
          {/* Sidebar */}
          <div className="w-56 bg-dark-800/20 border-r border-dark-800 p-3 flex flex-col gap-1.5 shrink-0 overflow-y-auto custom-scrollbar">
            <button 
              onClick={() => setActiveTab('mcp')}
              className={`flex items-center gap-2.5 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors ${activeTab === 'mcp' ? 'bg-brand-500/20 text-brand-400' : 'text-gray-400 hover:text-gray-200 hover:bg-dark-700/50'}`}
            >
              <Bot size={15} /> AI Agents (MCP)
            </button>
            
            <button 
              onClick={() => setActiveTab('mobile')}
              className={`flex items-center justify-between px-3 py-2.5 rounded-lg text-sm font-medium transition-colors ${activeTab === 'mobile' ? 'bg-purple-500/20 text-purple-300' : 'text-gray-400 hover:text-gray-200 hover:bg-dark-700/50'}`}
            >
              <span className="flex items-center gap-2.5">
                <Smartphone size={15} /> Glyph Mobile
              </span>
              <span className="text-[9px] font-mono font-bold uppercase px-1.5 py-0.5 rounded bg-purple-500/20 text-purple-300">
                APK
              </span>
            </button>
          </div>
          
          {/* Content */}
          <div className="flex-1 p-8 overflow-y-auto custom-scrollbar bg-dark-900/50">
            {activeTab === 'mcp' && (
              <div className="max-w-2xl">
                <div className="mb-6">
                  <h3 className="text-xl font-semibold text-gray-200 flex items-center gap-3">
                    <span className="relative flex h-3 w-3">
                      <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-50"></span>
                      <span className="relative inline-flex rounded-full h-3 w-3 bg-emerald-500"></span>
                    </span>
                    AI Agent Setup
                  </h3>
                  <p className="text-sm text-gray-400 mt-1.5">Connect your AI assistant via MCP to manage your servers seamlessly.</p>
                </div>
                
                {loading ? (
                  <div className="flex items-center justify-center py-12 text-gray-500 gap-2">
                    <RefreshCw size={18} className="animate-spin" />
                    <span className="text-sm">Scanning for AI clients…</span>
                  </div>
                ) : mcpInfo ? (
                  <div className="space-y-4">
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                      {Object.entries(CLIENTS).map(([clientId, meta]) => (
                        <ClientCard
                          key={clientId}
                          clientId={clientId}
                          meta={meta}
                          info={mcpInfo.clients[clientId] || { status: 'not-installed' }}
                          result={installResults[clientId]}
                          installing={installing}
                          onInstall={handleAutoInstall}
                          mcpScriptPath={mcpInfo.mcpScriptPath}
                        />
                      ))}
                    </div>

                    <ManualSetupPanel mcpScriptPath={mcpInfo.mcpScriptPath} />

                    <div className="flex justify-end pt-2">
                      <button onClick={loadInfo} disabled={loading} className="flex items-center gap-1.5 text-xs text-gray-500 hover:text-gray-300 transition-colors">
                        <RefreshCw size={12} className={loading ? 'animate-spin' : ''} /> Refresh status
                      </button>
                    </div>
                  </div>
                ) : (
                  <p className="text-sm text-gray-500 py-8 text-center bg-dark-800/30 rounded-xl border border-dark-700 border-dashed">Failed to load MCP info.</p>
                )}
              </div>
            )}

            {activeTab === 'mobile' && (
              <div className="max-w-2xl space-y-6">
                <div>
                  <h3 className="text-xl font-semibold text-gray-200 flex items-center gap-2.5">
                    <Smartphone size={22} className="text-purple-400" />
                    Glyph Mobile for Android
                  </h3>
                  <p className="text-sm text-gray-400 mt-1.5">
                    Manage your remote servers, terminals, and Docker containers anywhere from your pocket.
                  </p>
                </div>

                {/* Banner Card */}
                <div className="rounded-2xl border border-purple-500/30 bg-gradient-to-br from-purple-500/10 via-dark-800/60 to-brand-500/10 p-5">
                  <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="px-2 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider bg-purple-500/20 text-purple-300 border border-purple-500/30">
                          Android Release
                        </span>
                        <span className="text-xs text-gray-400 font-mono">v1.0.1 (APK Sideload)</span>
                      </div>
                      <h4 className="text-base font-semibold text-white mt-1.5">Scan to Download Instantly</h4>
                      <p className="text-xs text-gray-300 mt-1 max-w-md">
                        Scan the dynamic QR code with your Android camera to immediately download and install the latest release APK.
                      </p>
                    </div>

                    <button
                      onClick={onOpenMobile}
                      className="px-4 py-2.5 bg-gradient-to-r from-purple-600 to-brand-500 hover:from-purple-500 hover:to-brand-400 text-white text-xs font-semibold rounded-xl flex items-center gap-2 shadow-lg shadow-purple-500/20 transition-all shrink-0"
                    >
                      <QrCode size={16} /> Open QR Scanner
                    </button>
                  </div>
                </div>

                {/* Features Grid */}
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div className="p-3.5 rounded-xl border border-dark-700 bg-dark-800/50 flex gap-3 items-start">
                    <div className="p-2 rounded-lg bg-brand-500/10 text-brand-400 border border-brand-500/20 shrink-0">
                      <Cpu size={16} />
                    </div>
                    <div>
                      <h5 className="text-xs font-semibold text-gray-200">Live Hardware Telemetry</h5>
                      <p className="text-[11px] text-gray-400 mt-0.5">Real-time CPU, Memory, Disk, GPU, and Network bandwidth breakdown.</p>
                    </div>
                  </div>

                  <div className="p-3.5 rounded-xl border border-dark-700 bg-dark-800/50 flex gap-3 items-start">
                    <div className="p-2 rounded-lg bg-purple-500/10 text-purple-400 border border-purple-500/20 shrink-0">
                      <Terminal size={16} />
                    </div>
                    <div>
                      <h5 className="text-xs font-semibold text-gray-200">Interactive SSH Terminal</h5>
                      <p className="text-[11px] text-gray-400 mt-0.5">Mobile-optimized keyboard accessory bar (ESC, TAB, CTRL, ALT, ^C, /).</p>
                    </div>
                  </div>

                  <div className="p-3.5 rounded-xl border border-dark-700 bg-dark-800/50 flex gap-3 items-start">
                    <div className="p-2 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 shrink-0">
                      <Layers size={16} />
                    </div>
                    <div>
                      <h5 className="text-xs font-semibold text-gray-200">Docker & SFTP Editor</h5>
                      <p className="text-[11px] text-gray-400 mt-0.5">Manage containers, view logs, and edit remote configuration files on the go.</p>
                    </div>
                  </div>

                  <div className="p-3.5 rounded-xl border border-dark-700 bg-dark-800/50 flex gap-3 items-start">
                    <div className="p-2 rounded-lg bg-amber-500/10 text-amber-400 border border-amber-500/20 shrink-0">
                      <ShieldCheck size={16} />
                    </div>
                    <div>
                      <h5 className="text-xs font-semibold text-gray-200">Vault Interoperability</h5>
                      <p className="text-[11px] text-gray-400 mt-0.5">Seamlessly import your Desktop AES-256 encrypted server vault export.</p>
                    </div>
                  </div>
                </div>

                {/* Direct Action Links */}
                <div className="flex flex-wrap items-center gap-3 pt-2">
                  <button
                    onClick={() => window.open('https://github.com/TheLunatic1/glyph-app/releases/latest', '_blank')}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-dark-800 hover:bg-dark-700 border border-dark-700 text-xs font-medium text-gray-300 hover:text-white transition-all"
                  >
                    <ExternalLink size={13} /> View GitHub Releases
                  </button>
                  <button
                    onClick={() => window.open('https://github.com/TheLunatic1/glyph-app', '_blank')}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-dark-800 hover:bg-dark-700 border border-dark-700 text-xs font-medium text-gray-300 hover:text-white transition-all"
                  >
                    <ExternalLink size={13} /> Mobile Repository
                  </button>
                </div>
              </div>
            )}
          </div>
          
        </div>
      </div>
    </div>
  );
}
