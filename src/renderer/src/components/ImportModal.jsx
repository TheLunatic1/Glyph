import React, { useState, useEffect } from 'react';
import { Download, X, Key, Eye, EyeOff, Check, Server, FileText } from 'lucide-react';
import OsLogo from './OsLogo';

export default function ImportModal({ isOpen, onClose, onImportSuccess }) {
  const [step, setStep] = useState('password'); // 'password' | 'select'
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const [parsedServers, setParsedServers] = useState([]);
  const [selectedIndices, setSelectedIndices] = useState([]);

  useEffect(() => {
    if (isOpen) {
      setStep('password');
      setPassword('');
      setShowPassword(false);
      setError('');
      setLoading(false);
      setParsedServers([]);
      setSelectedIndices([]);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleReadEncryptedFile = async (e) => {
    e.preventDefault();
    if (!password) {
      setError('Master password is required to decrypt the import file');
      return;
    }

    setError('');
    setLoading(true);
    try {
      const serversList = await window.api.readImportFile(password);
      if (!serversList) {
        // User cancelled file picker dialog
        setLoading(false);
        return;
      }
      if (!Array.isArray(serversList) || serversList.length === 0) {
        setError('No valid servers found in the import file');
        setLoading(false);
        return;
      }

      setParsedServers(serversList);
      setSelectedIndices(serversList.map((_, idx) => idx));
      setStep('select');
    } catch (err) {
      setError(err.message || 'Failed to decrypt file. Check your password.');
    } finally {
      setLoading(false);
    }
  };

  const toggleSelect = (index) => {
    setSelectedIndices(prev =>
      prev.includes(index) ? prev.filter(i => i !== index) : [...prev, index]
    );
  };

  const toggleSelectAll = () => {
    if (selectedIndices.length === parsedServers.length) {
      setSelectedIndices([]);
    } else {
      setSelectedIndices(parsedServers.map((_, idx) => idx));
    }
  };

  const handleConfirmImport = async () => {
    if (selectedIndices.length === 0) {
      setError('Please select at least one server to import');
      return;
    }

    setLoading(true);
    setError('');
    try {
      const serversToImport = selectedIndices.map(i => parsedServers[i]);
      const count = await window.api.importSelectedServers(serversToImport);
      if (count > 0 && onImportSuccess) {
        onImportSuccess(count);
      }
      onClose();
    } catch (err) {
      setError(err.message || 'Failed to import servers');
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
              <Download size={20} />
            </div>
            <div>
              <h3 className="text-gray-100 font-semibold leading-tight">
                {step === 'password' ? 'Import Servers' : 'Select Servers to Import'}
              </h3>
              <p className="text-xs text-gray-400">
                {step === 'password'
                  ? 'Decrypt and load servers from an encrypted Glyph export file'
                  : `Found ${parsedServers.length} server(s) in backup file`}
              </p>
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

        {step === 'password' ? (
          /* Step 1: Master Password & File Selection */
          <form onSubmit={handleReadEncryptedFile} className="p-5 flex flex-col gap-4">
            <p className="text-sm text-gray-300">
              Enter the master password that was used to encrypt your backup file, then choose the <code className="text-brand-300 font-mono text-xs bg-dark-800 px-1.5 py-0.5 rounded">.glyph</code> export file.
            </p>

            <div>
              <label className="block text-xs font-semibold uppercase tracking-wider text-gray-400 mb-1.5 flex items-center gap-1.5">
                <Key size={14} className="text-brand-400" /> Master Decryption Password
              </label>
              <div className="relative">
                <input
                  type={showPassword ? 'text' : 'password'}
                  autoFocus
                  required
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="Master Password"
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
            </div>

            {error && (
              <div className="p-2.5 bg-red-500/10 border border-red-500/20 rounded-lg text-xs text-red-400">
                {error}
              </div>
            )}

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-dark-800">
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-2 bg-dark-800 hover:bg-dark-700 text-gray-300 rounded-lg transition-colors font-medium text-sm"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={loading || !password}
                className="px-5 py-2 bg-brand-500 hover:bg-brand-400 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg transition-colors font-medium text-sm shadow-lg shadow-brand-500/20 flex items-center gap-2"
              >
                <FileText size={16} />
                {loading ? 'Opening...' : 'Select File'}
              </button>
            </div>
          </form>
        ) : (
          /* Step 2: Selective Import */
          <div className="flex flex-col flex-1 overflow-hidden p-5 gap-4">
            <div className="flex items-center justify-between">
              <span className="text-xs font-semibold uppercase tracking-wider text-gray-400">
                Selected ({selectedIndices.length}/{parsedServers.length})
              </span>
              <button
                type="button"
                onClick={toggleSelectAll}
                className="text-xs text-brand-400 hover:text-brand-300 font-medium transition-colors"
              >
                {selectedIndices.length === parsedServers.length ? 'Deselect All' : 'Select All'}
              </button>
            </div>

            {/* Server List */}
            <div className="flex-1 overflow-y-auto max-h-56 custom-scrollbar border border-dark-700 rounded-xl bg-dark-950/40 divide-y divide-dark-800">
              {parsedServers.map((server, idx) => {
                const isSelected = selectedIndices.includes(idx);
                return (
                  <div
                    key={idx}
                    onClick={() => toggleSelect(idx)}
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
                      <p className="text-sm font-medium text-gray-200 truncate">{server.name || server.host}</p>
                      <p className="text-xs font-mono text-gray-400 truncate">
                        {server.username}@{server.host}:{server.port || 22}
                      </p>
                    </div>
                  </div>
                );
              })}
            </div>

            {error && (
              <div className="p-2.5 bg-red-500/10 border border-red-500/20 rounded-lg text-xs text-red-400">
                {error}
              </div>
            )}

            <div className="flex items-center justify-between pt-2 border-t border-dark-800 shrink-0">
              <button
                type="button"
                onClick={() => setStep('password')}
                className="text-xs text-gray-400 hover:text-gray-200 transition-colors"
              >
                ← Back
              </button>
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={onClose}
                  className="px-4 py-2 bg-dark-800 hover:bg-dark-700 text-gray-300 rounded-lg transition-colors font-medium text-sm"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  disabled={loading || selectedIndices.length === 0}
                  onClick={handleConfirmImport}
                  className="px-5 py-2 bg-brand-500 hover:bg-brand-400 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg transition-colors font-medium text-sm shadow-lg shadow-brand-500/20 flex items-center gap-2"
                >
                  <Download size={16} />
                  {loading ? 'Importing...' : `Import (${selectedIndices.length})`}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
