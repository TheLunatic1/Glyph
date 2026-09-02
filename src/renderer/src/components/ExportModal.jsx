import React, { useState, useEffect } from 'react';
import { Upload, X, Key, Eye, EyeOff, CheckSquare, Square, Check, Server } from 'lucide-react';
import OsLogo from './OsLogo';

export default function ExportModal({ isOpen, onClose, servers = [] }) {
  const [selectedIds, setSelectedIds] = useState([]);
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setSelectedIds(servers.map(s => s.id));
      setPassword('');
      setShowPassword(false);
      setError('');
      setLoading(false);
    }
  }, [isOpen, servers]);

  if (!isOpen) return null;

  const toggleSelect = (id) => {
    setSelectedIds(prev =>
      prev.includes(id) ? prev.filter(item => item !== id) : [...prev, id]
    );
  };

  const toggleSelectAll = () => {
    if (selectedIds.length === servers.length) {
      setSelectedIds([]);
    } else {
      setSelectedIds(servers.map(s => s.id));
    }
  };

  const handleExport = async (e) => {
    e.preventDefault();
    if (selectedIds.length === 0) {
      setError('Please select at least one server to export');
      return;
    }
    if (!password) {
      setError('Master password is required to encrypt the export');
      return;
    }

    setError('');
    setLoading(true);
    try {
      const success = await window.api.exportServers(password, selectedIds);
      if (success) {
        onClose();
      }
    } catch (err) {
      setError(err.message || 'Failed to export servers');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-fade-in">
      <div className="bg-dark-900 border border-dark-700 w-full max-w-lg rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-dark-800 bg-dark-800/30 shrink-0">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-lg bg-brand-500/10 border border-brand-500/20 flex items-center justify-center text-brand-400">
              <Upload size={20} />
            </div>
            <div>
              <h3 className="text-gray-100 font-semibold leading-tight">Export Servers</h3>
              <p className="text-xs text-gray-400">Export and encrypt your saved server configurations</p>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-1.5 text-gray-500 hover:text-gray-200 transition-colors rounded-lg hover:bg-dark-800"
          >
            <X size={18} />
          </button>
        </div>

        <form onSubmit={handleExport} className="flex flex-col flex-1 overflow-hidden p-5 gap-4">
          {/* Server Selection Header */}
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold uppercase tracking-wider text-gray-400">
              Select Servers ({selectedIds.length}/{servers.length})
            </span>
            <button
              type="button"
              onClick={toggleSelectAll}
              className="text-xs text-brand-400 hover:text-brand-300 font-medium transition-colors"
            >
              {selectedIds.length === servers.length ? 'Deselect All' : 'Select All'}
            </button>
          </div>

          {/* Server List */}
          <div className="flex-1 overflow-y-auto max-h-52 custom-scrollbar border border-dark-700 rounded-xl bg-dark-950/40 divide-y divide-dark-800">
            {servers.length === 0 ? (
              <div className="p-6 text-center text-gray-500 text-sm">
                No servers available to export.
              </div>
            ) : (
              servers.map((server) => {
                const isSelected = selectedIds.includes(server.id);
                return (
                  <div
                    key={server.id}
                    onClick={() => toggleSelect(server.id)}
                    className={`flex items-center gap-3 p-3 cursor-pointer transition-colors select-none ${
                      isSelected ? 'bg-brand-500/5 hover:bg-brand-500/10' : 'hover:bg-dark-800/40 opacity-70'
                    }`}
                  >
                    <div className="shrink-0 text-brand-400">
                      {isSelected ? (
                        <div className="w-4 h-4 rounded bg-brand-500 flex items-center justify-center text-white">
                          <Check size={12} strokeWidth={3} />
                        </div>
                      ) : (
                        <div className="w-4 h-4 rounded border border-dark-600 bg-dark-800" />
                      )}
                    </div>
                    <div className="w-8 h-8 shrink-0 rounded-full bg-dark-800 flex items-center justify-center border border-dark-700">
                      <OsLogo server={server} />
                    </div>
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium text-gray-200 truncate">{server.name}</p>
                      <p className="text-xs font-mono text-gray-400 truncate">
                        {server.username}@{server.host}:{server.port || 22}
                      </p>
                    </div>
                  </div>
                );
              })
            )}
          </div>

          {/* Password Input */}
          <div>
            <label className="block text-xs font-semibold uppercase tracking-wider text-gray-400 mb-1.5 flex items-center gap-1.5">
              <Key size={14} className="text-brand-400" /> Master Encryption Password
            </label>
            <div className="relative">
              <input
                type={showPassword ? 'text' : 'password'}
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Enter a master password to encrypt export..."
                className="w-full px-3 py-2 bg-dark-800 border border-dark-700 rounded-lg focus:outline-none focus:border-brand-500 text-gray-200 text-sm pr-10"
              />
              <button
                type="button"
                onClick={() => setShowPassword(!showPassword)}
                className="absolute inset-y-0 right-0 px-3 flex items-center text-gray-500 hover:text-gray-300 transition-colors"
                tabIndex="-1"
              >
                {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
              </button>
            </div>
            <p className="text-[11px] text-gray-500 mt-1">
              You will need this password when importing these servers on another machine.
            </p>
          </div>

          {error && (
            <div className="p-2.5 bg-red-500/10 border border-red-500/20 rounded-lg text-xs text-red-400">
              {error}
            </div>
          )}

          {/* Footer */}
          <div className="flex items-center justify-end gap-2 pt-2 border-t border-dark-800 shrink-0">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 bg-dark-800 hover:bg-dark-700 text-gray-300 rounded-lg transition-colors font-medium text-sm"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading || selectedIds.length === 0 || !password}
              className="px-5 py-2 bg-brand-500 hover:bg-brand-400 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg transition-colors font-medium text-sm shadow-lg shadow-brand-500/20 flex items-center gap-2"
            >
              <Upload size={16} />
              {loading ? 'Exporting...' : `Export (${selectedIds.length})`}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
