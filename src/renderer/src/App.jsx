import React, { useState, useEffect } from 'react';
import { Server, Plus, Play, Trash2, ShieldCheck, Terminal, HardDrive, Cpu, Search, X, ArrowRight, ExternalLink } from 'lucide-react';
import logoSrc from './assets/logo.png';
import Sidebar from './components/Sidebar';
import Dashboard from './pages/Dashboard';
import TerminalPage from './pages/Terminal';
import SFTP from './pages/SFTP';
import Commands from './pages/Commands';
import Containers from './pages/Containers';
import Secrets from './pages/Secrets';
import Tunnels from './pages/Tunnels';
import OsLogo from './components/OsLogo';
import UpdateModal from './components/UpdateModal';

const LiveTimer = ({ error }) => {
  const [ms, setMs] = useState(0);
  useEffect(() => {
    if (error) return;
    const start = Date.now();
    const intv = setInterval(() => setMs(Date.now() - start), 47);
    return () => clearInterval(intv);
  }, [error]);
  return <span className="font-mono text-xs bg-dark-800 px-2.5 py-1 rounded-lg text-brand-400 border border-brand-500/20 ml-3">{ms}ms</span>;
};

export default function App() {
  const [connected, setConnected] = useState(false);
  const [connectedServer, setConnectedServer] = useState(null);
  const [servers, setServers] = useState([]);
  const [showAddForm, setShowAddForm] = useState(false);
  const [activeTab, setActiveTab] = useState('dashboard');
  const [newServer, setNewServer] = useState({ name: '', host: '', username: '', password: '', port: 22, privateKey: '', zerotier: '' });
  const [connectingId, setConnectingId] = useState(null);
  const connectingIdRef = React.useRef(null);
  const [connectLogs, setConnectLogs] = useState([]);
  const [connectError, setConnectError] = useState(null);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [disconnectReason, setDisconnectReason] = useState(null);

  // ── Auto-update state ──────────────────────────────────────────────────────
  const [updateInfo, setUpdateInfo]       = useState(null);  // { version, releaseNotes, releaseDate }
  const [updateStage, setUpdateStage]     = useState('idle'); // idle | available | downloading | downloaded | error
  const [updateProgress, setUpdateProgress] = useState(null);
  const [updateError, setUpdateError]     = useState(null);
  const [showUpdateModal, setShowUpdateModal] = useState(false);
  
  const [ztNodeId, setZtNodeId] = useState('');
  const [appVersion, setAppVersion] = useState('');
  
  // Ref for auto-scrolling logs
  const logsEndRef = React.useRef(null);
  useEffect(() => {
    if (logsEndRef.current) logsEndRef.current.scrollIntoView({ behavior: 'smooth' });
  }, [connectLogs, connectError]);

  useEffect(() => {
    loadServers();
    window.api.getZtNodeId().then(id => { if (id) setZtNodeId(id); });
    window.api.getAppVersion().then(v => { if (v) setAppVersion(v); });

    // ── Auto-updater listeners (electron-updater via IPC) ────────────────────
    const removeAvailable = window.api.onUpdaterAvailable((info) => {
      setUpdateInfo(info);
      setUpdateError(null);
      setUpdateStage('available');
    });
    const removeProgress = window.api.onUpdaterProgress((prog) => {
      setUpdateProgress(prog);
      setUpdateStage('downloading');
    });
    const removeDownloaded = window.api.onUpdaterDownloaded(() => {
      setUpdateStage('downloaded');
      setShowUpdateModal(true); // pop open modal automatically when done
    });
    const removeError = window.api.onUpdaterError((msg) => {
      setUpdateError(msg);
      // If the user was actively interacting with an update ('available' or 'downloading'), switch to error state so Try Again / Manual buttons appear.
      setUpdateStage(prev => {
        if (prev !== 'idle') {
          setShowUpdateModal(true);
          return 'error';
        }
        return 'idle';
      });
      console.warn('[Updater] error:', msg);
    });

    // ── SSH status listener ────────────────────────────────────────────────
    const removeSshStatus = window.api.onSshStatus((msg) => {
      setConnectLogs(prev => [...prev, msg]);
    });

    // Fix #1: Listen for unexpected server-side disconnects
    const removeDisconnect = window.api.onSshDisconnected((reason) => {
      setConnected(false);
      setConnectedServer(null);
      setDisconnectReason(reason || 'Connection was lost.');
    });

    return () => {
      removeAvailable();
      removeProgress();
      removeDownloaded();
      removeError();
      removeSshStatus();
      removeDisconnect();
    };
  }, []);

  const loadServers = async () => {
    const list = await window.api.getServers();
    setServers(list);
  };

  const handleAddServer = async (e) => {
    e.preventDefault();
    await window.api.addServer(newServer);
    setShowAddForm(false);
    // Fix #3: Reset ALL fields including advanced ones
    setNewServer({ name: '', host: '', username: '', password: '', port: 22, privateKey: '', zerotier: '' });
    setShowAdvanced(false);
    loadServers();
  };

  const handleDelete = async (id, e) => {
    e.stopPropagation();
    await window.api.deleteServer(id);
    loadServers();
  };

  const handleConnect = async (id) => {
    if (connectingId !== null) return;
    setConnectingId(id);
    connectingIdRef.current = id;
    setConnectLogs(['Starting connection sequence...']);
    setConnectError(null);
    try {
      const res = await window.api.sshConnectSaved(id);
      if (connectingIdRef.current !== id) return; // User cancelled

      // Fix #2: sshConnectSaved resolves (not throws) on failure
      if (!res.success) {
        setConnectError(res.error || 'Connection failed.');
        setConnectingId(null);
        connectingIdRef.current = null;
        return;
      }

      setConnectLogs(prev => [...prev, 'Authentication successful, initializing session...']);
      const server = servers.find(s => s.id === id);
      setConnectedServer(server);
      setConnected(true);
      setActiveTab('dashboard');
      setConnectingId(null);
      connectingIdRef.current = null;
    } catch (err) {
      if (connectingIdRef.current !== id) return; // User cancelled
      let msg = 'Unknown error';
      if (err) {
        if (typeof err === 'string') msg = err;
        else if (err.error && typeof err.error === 'string') msg = err.error;
        else if (err.message) msg = err.message;
        else if (err.error && err.error.message) msg = err.error.message;
        else msg = JSON.stringify(err);
      }
      setConnectError(msg);
    } finally {
      loadServers();
    }
  };

  const handleCancelConnect = () => {
    connectingIdRef.current = null;
    setConnectingId(null);
    setConnectError(null); // Fix #12: clear error so it doesn't flash in next modal
    window.api.sshDisconnect();
  };

  if (!connected) {
    const activeConnectingServer = servers.find(s => s.id === connectingId);
    
    return (
      <div className="flex flex-col h-screen w-full bg-dark-900 overflow-y-auto relative">
        
        {/* Fix #1: Unexpected disconnect banner */}
        {disconnectReason && (
          <div className="z-50 bg-red-600 text-white px-4 py-2 flex items-center justify-center gap-4 shadow-lg relative">
            <span className="text-sm font-medium">⚠ {disconnectReason}</span>
            <button onClick={() => setDisconnectReason(null)} className="absolute right-4 top-1/2 -translate-y-1/2 p-1 rounded-md hover:bg-white/20 transition-colors">
              <X size={16} />
            </button>
          </div>
        )}

        {/* Update Banner — shown when electron-updater finds a new version */}
        {updateStage === 'available' && updateInfo && (
          <div className="z-50 bg-brand-500 text-white px-4 py-2 flex items-center justify-center gap-3 shadow-lg relative flex-wrap">
            <span className="text-sm font-medium">✨ Glyph v{updateInfo.version} is available!</span>
            <div className="flex gap-2">
              <button
                onClick={() => setShowUpdateModal(true)}
                className="px-3 py-1 bg-white/20 hover:bg-white/30 rounded-md text-xs font-semibold flex items-center gap-1 transition-colors"
              >
                View Release Notes & Update <ArrowRight size={13} />
              </button>
              <button
                onClick={() => window.open('https://github.com/TheLunatic1/Glyph/releases/latest', '_blank')}
                className="px-3 py-1 bg-white/10 hover:bg-white/20 rounded-md text-xs font-medium flex items-center gap-1 transition-colors"
              >
                <ExternalLink size={13} /> Download Manually
              </button>
            </div>
            <button onClick={() => setUpdateStage('idle')} className="absolute right-4 top-1/2 -translate-y-1/2 p-1 rounded-md hover:bg-white/20 transition-colors">
              <X size={16} />
            </button>
          </div>
        )}

        {/* Downloading banner — progress while update downloads in background */}
        {updateStage === 'downloading' && (
          <div className="z-50 bg-dark-800 border-b border-brand-500/30 text-white px-4 py-2 flex items-center justify-center gap-3 shadow-lg">
            <span className="text-sm font-medium text-brand-300">⬇ Downloading update... {updateProgress?.percent ?? 0}%</span>
            <button onClick={() => setShowUpdateModal(true)} className="px-3 py-1 bg-brand-500/20 hover:bg-brand-500/30 rounded-md text-xs font-medium transition-colors">
              View Progress
            </button>
          </div>
        )}

        {/* UpdateModal */}
        {showUpdateModal && (
          <UpdateModal
            info={updateInfo}
            stage={updateStage}
            progress={updateProgress}
            error={updateError}
            onDownload={async () => { setUpdateStage('downloading'); await window.api.updaterDownload(); }}
            onInstall={() => window.api.updaterInstall()}
            onRetry={async () => { setUpdateStage('downloading'); await window.api.updaterDownload(); }}
            onClose={() => setShowUpdateModal(false)}
          />
        )}

        {/* Connecting Modal */}
        {activeConnectingServer && (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
            <div className="bg-dark-900 border border-dark-700 w-full max-w-lg rounded-2xl shadow-2xl overflow-hidden flex flex-col">
              {/* Header */}
              <div className="flex items-center justify-between p-4 border-b border-dark-800 bg-dark-800/30">
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 shrink-0 rounded-full bg-dark-700 flex items-center justify-center border border-dark-600">
                    <OsLogo server={activeConnectingServer} />
                  </div>
                  <div>
                    <h3 className="text-gray-100 font-semibold leading-tight flex items-center">
                      {activeConnectingServer.name}
                      <LiveTimer error={connectError} />
                    </h3>
                    <p className="text-gray-400 text-xs font-mono">{activeConnectingServer.username}@{activeConnectingServer.host}:{activeConnectingServer.port || 22}</p>
                  </div>
                </div>
                <button onClick={handleCancelConnect} className="p-2 text-gray-500 hover:text-gray-200 transition-colors">
                  <X size={20}/>
                </button>
              </div>

              {/* Terminal Logs */}
              <div className="p-5 bg-[#0f111a] font-mono text-sm h-64 overflow-y-auto flex flex-col gap-1.5 custom-scrollbar">
                {connectLogs.map((log, i) => (
                  <div key={i} className="text-brand-300 flex gap-2">
                    <span className="text-dark-600 select-none">❯</span> {log}
                  </div>
                ))}
                {!connectError && (
                  <div className="text-gray-400 flex gap-2 animate-pulse">
                    <span className="text-dark-600 select-none">❯</span> <span className="w-2 h-4 bg-brand-500 inline-block mt-0.5"></span>
                  </div>
                )}
                {connectError && (
                  <div className="text-red-400 flex gap-2 mt-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg break-words">
                    <span className="text-red-500 select-none font-bold shrink-0">✖</span> <span>{connectError}</span>
                  </div>
                )}
                <div ref={logsEndRef} />
              </div>

              {/* Footer */}
              {connectError && (
                <div className="p-4 bg-dark-800/50 flex justify-end">
                  <button onClick={() => setConnectingId(null)} className="px-5 py-2 bg-dark-700 hover:bg-dark-600 text-gray-300 rounded-lg transition-colors font-medium">Close</button>
                </div>
              )}
            </div>
          </div>
        )}

        <header className="p-8 pb-0">
          <div className="flex items-center gap-3">
            <img src={logoSrc} alt="Glyph" className="w-11 h-11 rounded-xl object-contain" />
            <h1 className="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-gray-100 to-gray-400 tracking-widest">Glyph</h1>
          </div>
          <p className="text-gray-400 mt-2">Secure SSH & Server Management</p>
        </header>

        <main className="flex-1 p-8 max-w-6xl w-full mx-auto">
          <div className="flex items-center justify-between mb-8">
            <h2 className="text-2xl font-semibold text-gray-200">Saved Servers</h2>
            <button
              onClick={() => setShowAddForm(!showAddForm)}
              className="px-4 py-2 bg-brand-500 hover:bg-brand-400 text-white font-medium rounded-lg transition-colors flex items-center gap-2 shadow-lg shadow-brand-500/20"
            >
              <Plus size={18} /> Add Server
            </button>
          </div>

          {showAddForm && (
            <div className="glass-panel p-6 mb-8 border border-brand-500/30 bg-brand-500/5">
              <h3 className="text-lg font-medium text-gray-200 mb-4 flex items-center gap-2">
                <ShieldCheck className="text-brand-400" size={20} /> Add New Server (Secure Vault)
              </h3>
              <form onSubmit={handleAddServer} className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 items-end">
                <div>
                  <label className="block text-gray-400 text-sm mb-1">Name (Alias)</label>
                  <input required value={newServer.name} onChange={e => setNewServer({...newServer, name: e.target.value})} className="w-full px-3 py-2 bg-dark-900 border border-dark-700 rounded-lg focus:outline-none focus:border-brand-500" placeholder="Prod Server" />
                </div>
                <div>
                  <label className="block text-gray-400 text-sm mb-1">Host / IP</label>
                  <input required value={newServer.host} onChange={e => setNewServer({...newServer, host: e.target.value})} className="w-full px-3 py-2 bg-dark-900 border border-dark-700 rounded-lg focus:outline-none focus:border-brand-500" placeholder="192.168.1.100" />
                </div>
                <div>
                  <label className="block text-gray-400 text-sm mb-1">Port</label>
                  <input type="number" required value={newServer.port} onChange={e => setNewServer({...newServer, port: parseInt(e.target.value, 10)})} className="w-full px-3 py-2 bg-dark-900 border border-dark-700 rounded-lg focus:outline-none focus:border-brand-500" placeholder="22" />
                </div>
                <div>
                  <label className="block text-gray-400 text-sm mb-1">Username</label>
                  <input required value={newServer.username} onChange={e => setNewServer({...newServer, username: e.target.value})} className="w-full px-3 py-2 bg-dark-900 border border-dark-700 rounded-lg focus:outline-none focus:border-brand-500" placeholder="root" />
                </div>
                <div>
                  <label className="block text-gray-400 text-sm mb-1">Password</label>
                  <input type="password" required value={newServer.password} onChange={e => setNewServer({...newServer, password: e.target.value})} className="w-full px-3 py-2 bg-dark-900 border border-dark-700 rounded-lg focus:outline-none focus:border-brand-500" placeholder="••••••••" />
                </div>
                {showAdvanced && (
                  <div className="lg:col-span-4 bg-dark-800/50 p-4 rounded-xl border border-brand-500/10 grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                      <label className="block text-gray-400 text-sm mb-1">ZeroTier Network ID <span className="text-gray-500">(Optional)</span></label>
                      <input value={newServer.zerotier || ''} onChange={e => setNewServer({...newServer, zerotier: e.target.value})} className="w-full px-3 py-2 bg-dark-900 border border-dark-700 rounded-lg focus:outline-none focus:border-brand-500 font-mono text-sm" placeholder="e5cd7a9e1cae134f" />
                      {ztNodeId && (
                        <p className="text-xs text-gray-500 mt-1.5 flex items-center gap-1">
                          <span className="w-1.5 h-1.5 rounded-full bg-brand-500/50"></span>
                          Your ZT Node ID: <span className="font-mono text-gray-300 font-medium select-all">{ztNodeId}</span> (authorize this in ZT Central)
                        </p>
                      )}
                    </div>
                    <div>
                      <label className="block text-gray-400 text-sm mb-1">SSH Private Key Path <span className="text-gray-500">(Optional)</span></label>
                      <input value={newServer.privateKey || ''} onChange={e => setNewServer({...newServer, privateKey: e.target.value})} className="w-full px-3 py-2 bg-dark-900 border border-dark-700 rounded-lg focus:outline-none focus:border-brand-500 font-mono text-sm" placeholder="/home/user/.ssh/id_rsa" />
                      <p className="text-xs text-gray-500 mt-1.5">Absolute path to your private key file on <em>this</em> machine</p>
                    </div>
                  </div>
                )}
                
                <div className="lg:col-span-4 flex justify-between items-center mt-2">
                  <button type="button" onClick={() => setShowAdvanced(!showAdvanced)} className="text-sm text-gray-400 hover:text-brand-400 transition-colors">
                    {showAdvanced ? 'Hide Advanced Options' : 'Show Advanced Options'}
                  </button>
                  <div className="flex gap-2">
                    <button type="submit" className="px-6 py-2 bg-brand-500 hover:bg-brand-400 text-white rounded-lg transition-colors font-medium">Save</button>
                    <button type="button" onClick={() => { setShowAddForm(false); setShowAdvanced(false); setNewServer({ name: '', host: '', username: '', password: '', port: 22, privateKey: '', zerotier: '' }); }} className="px-4 py-2 bg-dark-700 hover:bg-dark-600 text-gray-300 rounded-lg transition-colors">Cancel</button>
                  </div>
                </div>
              </form>
            </div>
          )}

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {servers.length === 0 && !showAddForm && (
              <div className="col-span-full py-16 text-center border-2 border-dashed border-dark-700 rounded-2xl">
                <Server className="mx-auto text-dark-600 mb-4" size={48} />
                <h3 className="text-xl font-medium text-gray-400">No servers saved</h3>
                <p className="text-gray-500 mt-2">Add a server to get started</p>
              </div>
            )}
            {servers.map(server => (
              <div
                key={server.id}
                onClick={() => handleConnect(server.id)}
                className={`glass-panel p-6 group hover:border-brand-500/50 transition-all hover:shadow-brand-500/10 hover:-translate-y-1 relative ${
                  connectingId !== null ? 'cursor-not-allowed opacity-70' : 'cursor-pointer'
                }`}
              >
                <button
                  onClick={(e) => handleDelete(server.id, e)}
                  className="absolute top-4 right-4 p-2 text-gray-500 hover:text-red-400 hover:bg-red-500/10 rounded-lg opacity-0 group-hover:opacity-100 transition-all"
                >
                  <Trash2 size={18} />
                </button>
                <div className="flex items-center gap-4 mb-4">
                  <div className="w-12 h-12 shrink-0 rounded-full bg-dark-700 overflow-hidden flex items-center justify-center border border-dark-600 group-hover:border-brand-500 transition-colors">
                    <OsLogo server={server} />
                  </div>
                  <div>
                    <h3 className="text-lg font-semibold text-gray-100">{server.name}</h3>
                    <p className="text-gray-400 text-sm font-mono">{server.username}@{server.host}:{server.port || 22}</p>
                  </div>
                </div>
                <div className="flex items-center justify-between mt-6">
                  <div className="flex gap-2">
                    {/* Fix #13: Only show green OS badge if OS is known; fallback is neutral */}
                    <span className={`text-xs font-medium px-2.5 py-1 rounded-full flex items-center gap-1.5 ${
                      server.os
                        ? 'bg-green-500/10 text-green-400'
                        : 'bg-dark-700 text-gray-500'
                    }`}>
                      <div className={`w-1.5 h-1.5 rounded-full ${server.os ? 'bg-green-400' : 'bg-gray-500'}`}></div>
                      {server.os ? server.os.toUpperCase() : 'Saved'}
                    </span>
                    {server.zerotier && (
                      <span className="text-xs font-medium px-2.5 py-1 bg-yellow-500/10 text-yellow-400 rounded-full flex items-center gap-1.5">
                        <div className="w-1.5 h-1.5 rounded-full bg-yellow-400"></div>
                        ZT Network
                      </span>
                    )}
                  </div>
                  <button className="flex items-center gap-2 text-sm font-medium text-brand-400 group-hover:text-brand-300">
                    Connect <Play size={14} fill="currentColor" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </main>
        
        <footer className="py-4 text-center text-xs text-gray-500 font-medium">
          Made by <a href="https://github.com/TheLunatic1" target="_blank" rel="noopener noreferrer" className="text-brand-400 hover:text-brand-300 transition-colors">TheLunatic1 (Salman Toha)</a>
          {appVersion && <span className="ml-2 text-gray-600 font-mono">v{appVersion}</span>}
        </footer>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen w-full bg-dark-900 overflow-hidden relative">
      {/* Update Banner — available */}
      {updateStage === 'available' && updateInfo && (
        <div className="z-50 bg-brand-500 text-white px-4 py-2 flex items-center justify-center gap-3 shadow-lg relative flex-wrap shrink-0">
          <span className="text-sm font-medium">✨ Glyph v{updateInfo.version} is available!</span>
          <div className="flex gap-2">
            <button
              onClick={() => setShowUpdateModal(true)}
              className="px-3 py-1 bg-white/20 hover:bg-white/30 rounded-md text-xs font-semibold flex items-center gap-1 transition-colors"
            >
              View Release Notes & Update <ArrowRight size={13} />
            </button>
            <button
              onClick={() => window.open('https://github.com/TheLunatic1/Glyph/releases/latest', '_blank')}
              className="px-3 py-1 bg-white/10 hover:bg-white/20 rounded-md text-xs font-medium flex items-center gap-1 transition-colors"
            >
              <ExternalLink size={13} /> Download Manually
            </button>
          </div>
          <button onClick={() => setUpdateStage('idle')} className="absolute right-4 top-1/2 -translate-y-1/2 p-1 rounded-md hover:bg-white/20 transition-colors">
            <X size={16} />
          </button>
        </div>
      )}
      {/* Downloading banner */}
      {updateStage === 'downloading' && (
        <div className="z-50 bg-dark-800 border-b border-brand-500/30 text-white px-4 py-2 flex items-center justify-center gap-3 shadow-lg shrink-0">
          <span className="text-sm font-medium text-brand-300">⬇ Downloading update... {updateProgress?.percent ?? 0}%</span>
          <button onClick={() => setShowUpdateModal(true)} className="px-3 py-1 bg-brand-500/20 hover:bg-brand-500/30 rounded-md text-xs font-medium transition-colors">
            View Progress
          </button>
        </div>
      )}
      {/* UpdateModal */}
      {showUpdateModal && (
        <UpdateModal
          info={updateInfo}
          stage={updateStage}
          progress={updateProgress}
          error={updateError}
          onDownload={async () => { setUpdateStage('downloading'); await window.api.updaterDownload(); }}
          onInstall={() => window.api.updaterInstall()}
          onRetry={async () => { setUpdateStage('downloading'); await window.api.updaterDownload(); }}
          onClose={() => setShowUpdateModal(false)}
        />
      )}
      <div className="flex flex-1 overflow-hidden">
        <Sidebar
          activeTab={activeTab}
          onTabChange={setActiveTab}
          onDisconnect={async () => {
            await window.api.sshDisconnect();
            setConnected(false);
            setConnectedServer(null);
            loadServers();
          }}
        />
        <main className="flex-1 flex flex-col h-full overflow-hidden">
        {/* All panels stay mounted — terminal keeps its session. CSS controls visibility */}
        <div style={{ display: activeTab === 'dashboard' ? 'flex' : 'none', flexDirection: 'column', flex: 1, height: '100%', overflow: 'hidden' }}>
          <Dashboard server={connectedServer} />
        </div>
        <div style={{ display: activeTab === 'terminal' ? 'flex' : 'none', flexDirection: 'column', flex: 1, height: '100%', overflow: 'hidden' }}>
          <TerminalPage server={connectedServer} />
        </div>
        <div style={{ display: activeTab === 'sftp' ? 'flex' : 'none', flexDirection: 'column', flex: 1, height: '100%', overflow: 'hidden' }}>
          <SFTP />
        </div>
        <div style={{ display: activeTab === 'commands' ? 'flex' : 'none', flexDirection: 'column', flex: 1, height: '100%', overflow: 'hidden' }}>
          <Commands server={connectedServer} />
        </div>
        <div style={{ display: activeTab === 'secrets' ? 'flex' : 'none', flexDirection: 'column', flex: 1, height: '100%', overflow: 'hidden' }}>
          <Secrets />
        </div>
        <div style={{ display: activeTab === 'tunnels' ? 'flex' : 'none', flexDirection: 'column', flex: 1, height: '100%', overflow: 'hidden' }}>
          <Tunnels />
        </div>
        <div style={{ display: activeTab === 'containers' ? 'flex' : 'none', flexDirection: 'column', flex: 1, height: '100%', overflow: 'hidden' }}>
          <Containers />
        </div>
      </main>
      </div>
    </div>
  );
}
