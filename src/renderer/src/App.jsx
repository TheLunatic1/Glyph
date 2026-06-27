import React, { useState, useEffect } from 'react';
import { Server, Plus, Play, Trash2, ShieldCheck, Terminal, HardDrive, Cpu, Search } from 'lucide-react';
import logoSrc from './assets/logo.png';
import Sidebar from './components/Sidebar';
import Dashboard from './pages/Dashboard';
import TerminalPage from './pages/Terminal';
import SFTP from './pages/SFTP';
import Commands from './pages/Commands';
import Containers from './pages/Containers';
import OsLogo from './components/OsLogo';

export default function App() {
  const [connected, setConnected] = useState(false);
  const [connectedServer, setConnectedServer] = useState(null);
  const [servers, setServers] = useState([]);
  const [showAddForm, setShowAddForm] = useState(false);
  const [activeTab, setActiveTab] = useState('dashboard');
  const [newServer, setNewServer] = useState({ name: '', host: '', username: '', password: '', port: 22 });
  const [connectingId, setConnectingId] = useState(null);
  const [showAdvanced, setShowAdvanced] = useState(false);

  const [ztNodeId, setZtNodeId] = useState('');

  useEffect(() => {
    loadServers();
    window.api.getZtNodeId().then(id => { if (id) setZtNodeId(id); });
  }, []);

  const loadServers = async () => {
    const list = await window.api.getServers();
    setServers(list);
  };

  const handleAddServer = async (e) => {
    e.preventDefault();
    await window.api.addServer(newServer);
    setShowAddForm(false);
    setNewServer({ name: '', host: '', username: '', password: '', port: 22 });
    loadServers();
  };

  const handleDelete = async (id, e) => {
    e.stopPropagation();
    await window.api.deleteServer(id);
    loadServers();
  };

  const handleConnect = async (id) => {
    setConnectingId(id);
    try {
      const res = await window.api.sshConnectSaved(id);
      if (res.success) {
        const server = servers.find(s => s.id === id);
        setConnectedServer(server);
        setConnected(true);
        setActiveTab('dashboard');
      }
    } catch (err) {
      alert('Connection failed: ' + (err.error || err.message));
    } finally {
      setConnectingId(null);
      loadServers();
    }
  };

  if (!connected) {
    return (
      <div className="flex flex-col h-screen w-full bg-dark-900 overflow-y-auto">
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
                  <div className="lg:col-span-4 bg-dark-800/50 p-4 rounded-xl border border-brand-500/10">
                    <label className="block text-gray-400 text-sm mb-1">ZeroTier Network ID (Optional)</label>
                    <input value={newServer.zerotier || ''} onChange={e => setNewServer({...newServer, zerotier: e.target.value})} className="w-full px-3 py-2 bg-dark-900 border border-dark-700 rounded-lg focus:outline-none focus:border-brand-500 font-mono text-sm" placeholder="e5cd7a9e1cae134f" />
                    {ztNodeId && (
                      <p className="text-xs text-gray-500 mt-1.5 flex items-center gap-1">
                        <span className="w-1.5 h-1.5 rounded-full bg-brand-500/50"></span>
                        Your ZT Node ID: <span className="font-mono text-gray-300 font-medium select-all">{ztNodeId}</span> (authorize this in ZT Central)
                      </p>
                    )}
                  </div>
                )}
                
                <div className="lg:col-span-4 flex justify-between items-center mt-2">
                  <button type="button" onClick={() => setShowAdvanced(!showAdvanced)} className="text-sm text-gray-400 hover:text-brand-400 transition-colors">
                    {showAdvanced ? 'Hide Advanced Options' : 'Show Advanced Options'}
                  </button>
                  <div className="flex gap-2">
                    <button type="submit" className="px-6 py-2 bg-brand-500 hover:bg-brand-400 text-white rounded-lg transition-colors font-medium">Save</button>
                    <button type="button" onClick={() => { setShowAddForm(false); setShowAdvanced(false); }} className="px-4 py-2 bg-dark-700 hover:bg-dark-600 text-gray-300 rounded-lg transition-colors">Cancel</button>
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
                className="glass-panel p-6 group hover:border-brand-500/50 cursor-pointer transition-all hover:shadow-brand-500/10 hover:-translate-y-1 relative"
              >
                <button
                  onClick={(e) => handleDelete(server.id, e)}
                  className="absolute top-4 right-4 p-2 text-gray-500 hover:text-red-400 hover:bg-red-500/10 rounded-lg opacity-0 group-hover:opacity-100 transition-all"
                >
                  <Trash2 size={18} />
                </button>
                <div className="flex items-center gap-4 mb-4">
                  <div className="w-12 h-12 rounded-full bg-dark-700 overflow-hidden flex items-center justify-center border border-dark-600 group-hover:border-brand-500 transition-colors">
                    <OsLogo server={server} />
                  </div>
                  <div>
                    <h3 className="text-lg font-semibold text-gray-100">{server.name}</h3>
                    <p className="text-gray-400 text-sm font-mono">{server.username}@{server.host}:{server.port || 22}</p>
                  </div>
                </div>
                <div className="flex items-center justify-between mt-6">
                  <div className="flex gap-2">
                    <span className="text-xs font-medium px-2.5 py-1 bg-green-500/10 text-green-400 rounded-full flex items-center gap-1.5">
                      <div className="w-1.5 h-1.5 rounded-full bg-green-400"></div>
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
                    {connectingId === server.id ? 'Connecting...' : 'Connect'} <Play size={14} fill="currentColor" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </main>
        
        <footer className="py-4 text-center text-xs text-gray-500 font-medium">
          Made by <a href="https://github.com/TheLunatic1" target="_blank" rel="noopener noreferrer" className="text-brand-400 hover:text-brand-300 transition-colors">TheLunatic1 (Salman Toha)</a>
        </footer>
      </div>
    );
  }

  return (
    <div className="flex h-screen w-full bg-dark-900 overflow-hidden">
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
          <TerminalPage />
        </div>
        <div style={{ display: activeTab === 'sftp' ? 'flex' : 'none', flexDirection: 'column', flex: 1, height: '100%', overflow: 'hidden' }}>
          <SFTP />
        </div>
        <div style={{ display: activeTab === 'commands' ? 'flex' : 'none', flexDirection: 'column', flex: 1, height: '100%', overflow: 'hidden' }}>
          <Commands />
        </div>
        <div style={{ display: activeTab === 'containers' ? 'flex' : 'none', flexDirection: 'column', flex: 1, height: '100%', overflow: 'hidden' }}>
          <Containers />
        </div>
      </main>
    </div>
  );
}
