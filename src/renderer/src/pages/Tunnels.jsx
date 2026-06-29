import React, { useState, useEffect } from 'react';
import { Network, Plus, Trash2, ShieldCheck, ArrowRight, Server, RefreshCw } from 'lucide-react';

export default function Tunnels() {
  const [tunnels, setTunnels] = useState([]);
  const [localPort, setLocalPort] = useState('');
  const [remoteHost, setRemoteHost] = useState('127.0.0.1');
  const [remotePort, setRemotePort] = useState('');
  const [loading, setLoading] = useState(false);
  const [deletingPort, setDeletingPort] = useState(null);
  const [error, setError] = useState(null);

  const fetchTunnels = async () => {
    try {
      const active = await window.api.sshGetTunnels();
      setTunnels(active);
    } catch (err) {
      console.error('Failed to fetch tunnels:', err);
    }
  };

  useEffect(() => {
    fetchTunnels();
    const intv = setInterval(fetchTunnels, 2000);
    return () => clearInterval(intv);
  }, []);

  const handleStartTunnel = async (e) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      await window.api.sshStartTunnel(parseInt(localPort), remoteHost, parseInt(remotePort));
      setLocalPort('');
      setRemotePort('');
      fetchTunnels();
    } catch (err) {
      setError(err.message || String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleStopTunnel = async (port) => {
    setDeletingPort(port);
    try {
      await window.api.sshStopTunnel(port);
      fetchTunnels();
    } catch (err) {
      console.error('Failed to stop tunnel:', err);
      alert('Failed to stop tunnel');
    } finally {
      setDeletingPort(null);
    }
  };

  return (
    <div className="p-8 h-full flex flex-col overflow-y-auto">
      <header className="mb-8">
        <h2 className="text-3xl font-bold text-gray-100 mb-2 tracking-wide flex items-center gap-3">
          <Network className="text-brand-400" size={32} />
          Port Forwarding
        </h2>
        <p className="text-gray-400 text-sm max-w-2xl leading-relaxed">
          Create secure SSH tunnels to access internal remote services directly from your local machine. 
          For example, bind local port 3306 to remote 127.0.0.1:3306 to access a protected database.
        </p>
      </header>

      {/* Add Tunnel Form */}
      <div className="bg-dark-800 border border-dark-700 rounded-xl p-6 shadow-xl mb-8">
        <h3 className="text-lg font-semibold text-gray-200 mb-4 flex items-center gap-2">
          <Plus size={20} className="text-brand-400" />
          Create New Tunnel
        </h3>
        
        {error && (
          <div className="bg-red-500/10 border border-red-500/20 text-red-400 p-3 rounded-lg text-sm mb-4">
            {error}
          </div>
        )}

        <form onSubmit={handleStartTunnel} className="flex items-end gap-4">
          <div className="flex-1">
            <label className="block text-xs font-medium text-gray-400 mb-2 uppercase tracking-wider">Local Port</label>
            <input 
              type="number" 
              required
              min="1"
              max="65535"
              placeholder="e.g. 3306"
              value={localPort}
              onChange={e => setLocalPort(e.target.value)}
              className="w-full bg-dark-900 border border-dark-700 rounded-lg px-4 py-2.5 text-gray-100 focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500 transition-all font-mono"
            />
          </div>
          
          <div className="mb-3">
            <ArrowRight className="text-gray-600" size={24} />
          </div>

          <div className="flex-1">
            <label className="block text-xs font-medium text-gray-400 mb-2 uppercase tracking-wider">Remote Host</label>
            <input 
              type="text" 
              required
              placeholder="e.g. 127.0.0.1"
              value={remoteHost}
              onChange={e => setRemoteHost(e.target.value)}
              className="w-full bg-dark-900 border border-dark-700 rounded-lg px-4 py-2.5 text-gray-100 focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500 transition-all font-mono"
            />
          </div>

          <div className="flex-1">
            <label className="block text-xs font-medium text-gray-400 mb-2 uppercase tracking-wider">Remote Port</label>
            <input 
              type="number" 
              required
              min="1"
              max="65535"
              placeholder="e.g. 3306"
              value={remotePort}
              onChange={e => setRemotePort(e.target.value)}
              className="w-full bg-dark-900 border border-dark-700 rounded-lg px-4 py-2.5 text-gray-100 focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500 transition-all font-mono"
            />
          </div>

          <button 
            type="submit"
            disabled={loading || !localPort || !remotePort || !remoteHost}
            className="px-6 py-2.5 bg-brand-500 hover:bg-brand-400 disabled:opacity-50 text-white font-semibold rounded-lg flex items-center gap-2 transition-colors shadow-lg shadow-brand-500/20"
          >
            {loading ? <RefreshCw className="animate-spin" size={20} /> : <ShieldCheck size={20} />}
            Bind Tunnel
          </button>
        </form>
      </div>

      {/* Active Tunnels List */}
      <h3 className="text-lg font-semibold text-gray-200 mb-4 flex items-center gap-2">
        <Server size={20} className="text-brand-400" />
        Active Tunnels
      </h3>
      
      {tunnels.length === 0 ? (
        <div className="text-center py-16 bg-dark-800/30 border border-dark-700/50 rounded-xl border-dashed">
          <Network className="text-gray-600 mx-auto mb-3" size={48} />
          <p className="text-gray-500">No active SSH tunnels. Create one above to get started.</p>
        </div>
      ) : (
        <div className="bg-dark-800 border border-dark-700 rounded-xl overflow-hidden shadow-xl">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="bg-dark-900/50 border-b border-dark-700">
                <th className="px-6 py-4 text-xs font-semibold text-gray-400 uppercase tracking-wider">Local Endpoint</th>
                <th className="px-6 py-4 text-xs font-semibold text-gray-400 uppercase tracking-wider">Remote Destination</th>
                <th className="px-6 py-4 text-xs font-semibold text-gray-400 uppercase tracking-wider">Status</th>
                <th className="px-6 py-4 text-right"></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-dark-700">
              {tunnels.map((t) => (
                <tr key={t.localPort} className="hover:bg-dark-700/30 transition-colors">
                  <td className="px-6 py-4">
                    <button 
                      onClick={() => window.open(`http://localhost:${t.localPort}`, '_blank')}
                      className="font-mono text-gray-200 hover:text-brand-300 hover:underline transition-all text-left flex items-center gap-2 group"
                      title="Open in Browser"
                    >
                      <span className="text-brand-400 font-semibold group-hover:text-brand-300 transition-colors">localhost:</span>{t.localPort}
                    </button>
                  </td>
                  <td className="px-6 py-4">
                    <div className="font-mono text-gray-400 flex items-center gap-2">
                      <ArrowRight size={14} className="text-gray-600" />
                      {t.remoteHost}:{t.remotePort}
                    </div>
                  </td>
                  <td className="px-6 py-4">
                    <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                      <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                      Active
                    </span>
                  </td>
                  <td className="px-6 py-4 text-right">
                    <button 
                      onClick={() => handleStopTunnel(t.localPort)}
                      disabled={deletingPort === t.localPort}
                      className="p-2 text-red-400 hover:text-red-300 hover:bg-red-500/10 rounded-lg transition-colors disabled:opacity-50"
                      title="Stop Tunnel"
                    >
                      {deletingPort === t.localPort ? <RefreshCw className="animate-spin" size={18} /> : <Trash2 size={18} />}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
