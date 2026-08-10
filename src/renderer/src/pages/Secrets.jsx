import React, { useState, useEffect } from 'react';
import { Lock, Plus, Trash2, Key, AlertTriangle, Download, Upload, X, Eye, EyeOff } from 'lucide-react';

export default function Secrets({ server }) {
  const [secrets, setSecrets] = useState([]);
  const [name, setName] = useState('');
  const [value, setValue] = useState('');
  const [encryptionAvailable, setEncryptionAvailable] = useState(true);
  
  const [showPasswordModal, setShowPasswordModal] = useState({ visible: false, action: null });
  const [masterPassword, setMasterPassword] = useState('');
  const [passwordError, setPasswordError] = useState('');
  const [showMasterPassword, setShowMasterPassword] = useState(false);

  const loadSecrets = async () => {
    if (!server?.id) return;
    const list = await window.api.getSecrets(server.id);
    setSecrets(list);
  };

  useEffect(() => {
    loadSecrets();
    window.api.isEncryptionAvailable().then(available => {
      setEncryptionAvailable(available);
    });

    const onSecretsUpdated = () => {
      loadSecrets();
    };
    window.addEventListener('secretsUpdated', onSecretsUpdated);
    return () => {
      window.removeEventListener('secretsUpdated', onSecretsUpdated);
    };
  }, [server]);

  const handleAdd = async (e) => {
    e.preventDefault();
    if (!name || !value || !server?.id) return;
    await window.api.addSecret(server.id, name, value);
    setName('');
    setValue('');
    loadSecrets();
  };

  const handleDelete = async (id) => {
    await window.api.deleteSecret(id);
    loadSecrets();
  };

  const handlePasswordSubmit = async (e) => {
    e.preventDefault();
    if (!masterPassword) {
      setPasswordError('Password is required');
      return;
    }
    
    setPasswordError('');
    try {
      if (showPasswordModal.action === 'export') {
        const success = await window.api.exportSecrets(server.id, masterPassword);
        if (success) {
          // Modal auto closes on success
        }
      } else if (showPasswordModal.action === 'import') {
        const count = await window.api.importSecrets(server.id, masterPassword);
        if (count !== false) {
          loadSecrets(); // Refresh list after import
        }
      }
      setShowPasswordModal({ visible: false, action: null });
      setMasterPassword('');
    } catch (err) {
      setPasswordError(err.message || 'Operation failed');
    }
  };

  return (
    <div className="p-8 h-full flex flex-col relative">
      {/* Password Modal */}
      {showPasswordModal.visible && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-dark-900 border border-dark-700 w-full max-w-sm rounded-2xl shadow-2xl overflow-hidden flex flex-col">
            <div className="flex items-center justify-between p-4 border-b border-dark-800 bg-dark-800/30">
              <div className="flex items-center gap-3">
                <Key className="text-brand-400" size={20} />
                <h3 className="text-gray-100 font-semibold">{showPasswordModal.action === 'export' ? 'Export Master Password' : 'Import Master Password'}</h3>
              </div>
              <button onClick={() => { setShowPasswordModal({ visible: false, action: null }); setPasswordError(''); setMasterPassword(''); }} className="p-2 text-gray-500 hover:text-gray-200 transition-colors">
                <X size={20}/>
              </button>
            </div>
            <form onSubmit={handlePasswordSubmit} className="p-5 flex flex-col gap-4">
              {showPasswordModal.action === 'export' ? (
                <p className="text-sm text-gray-400">Enter a master password to securely encrypt your secrets. You will need this password to import them later.</p>
              ) : (
                <p className="text-sm text-gray-400">Enter the master password that was used to encrypt the export file.</p>
              )}
              <div>
                <div className="relative">
                  <input
                    type={showMasterPassword ? "text" : "password"}
                    autoFocus
                    required
                    value={masterPassword}
                    onChange={(e) => setMasterPassword(e.target.value)}
                    placeholder="Master Password"
                    className="w-full px-3 py-2 bg-dark-800 border border-dark-700 rounded-lg focus:outline-none focus:border-brand-500 text-gray-200 pr-10"
                  />
                  <button
                    type="button"
                    onClick={() => setShowMasterPassword(!showMasterPassword)}
                    className="absolute inset-y-0 right-0 px-3 flex items-center text-gray-500 hover:text-gray-300 transition-colors"
                    tabIndex="-1"
                  >
                    {showMasterPassword ? <EyeOff size={18} /> : <Eye size={18} />}
                  </button>
                </div>
                {passwordError && <p className="text-xs text-red-400 mt-2">{passwordError}</p>}
              </div>
              <div className="flex justify-end gap-2 mt-2">
                <button type="button" onClick={() => { setShowPasswordModal({ visible: false, action: null }); setPasswordError(''); setMasterPassword(''); }} className="px-4 py-2 bg-dark-800 hover:bg-dark-700 text-gray-300 rounded-lg transition-colors font-medium text-sm">Cancel</button>
                <button type="submit" className="px-4 py-2 bg-brand-500 hover:bg-brand-400 text-white rounded-lg transition-colors font-medium text-sm">{showPasswordModal.action === 'export' ? 'Export' : 'Select File'}</button>
              </div>
            </form>
          </div>
        </div>
      )}

      <header className="mb-8 flex justify-between items-start">
        <div>
          <h2 className="text-3xl font-bold text-gray-100 flex items-center gap-3">
            <Lock className="text-brand-400" size={28} /> Secrets Vault
          </h2>
          <p className="text-gray-400 mt-2 max-w-2xl">
          Store highly sensitive tokens (e.g. GitHub API keys) encrypted on disk for {server?.name}. These can be securely injected into any active terminal session without exposing the plaintext.
        </p>
        {!encryptionAvailable && (
          <div className="mt-3 flex items-start gap-3 p-3 bg-yellow-500/10 border border-yellow-500/30 rounded-xl text-yellow-400 text-sm">
            <AlertTriangle size={18} className="shrink-0 mt-0.5" />
            <span>
              <strong>Warning:</strong> Your system's keychain is unavailable. Secrets will be stored as Base64 (not encrypted). Avoid storing highly sensitive values until this is resolved.
            </span>
          </div>
        )}
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={() => setShowPasswordModal({ visible: true, action: 'import' })}
            className="p-2 text-gray-400 hover:text-brand-400 hover:bg-brand-500/10 rounded-lg transition-colors"
            title="Import Secrets"
          >
            <Download size={20} />
          </button>
          <button
            onClick={() => setShowPasswordModal({ visible: true, action: 'export' })}
            className="p-2 text-gray-400 hover:text-brand-400 hover:bg-brand-500/10 rounded-lg transition-colors"
            title="Export Secrets"
          >
            <Upload size={20} />
          </button>
        </div>
      </header>

      <form onSubmit={handleAdd} className="mb-8 glass-panel p-6 flex gap-4 items-end border border-brand-500/30 bg-brand-500/5">
        <div className="flex-1">
          <label className="block text-gray-400 text-sm mb-2">Secret Alias</label>
          <input
            required
            value={name}
            onChange={e => setName(e.target.value)}
            className="w-full px-4 py-2 bg-dark-900 border border-dark-700 rounded-lg focus:outline-none focus:border-brand-500"
            placeholder="e.g. GitHub Prod Token"
          />
        </div>
        <div className="flex-1">
          <label className="block text-gray-400 text-sm mb-2">Secret Value</label>
          <input
            required
            type="password"
            value={value}
            onChange={e => setValue(e.target.value)}
            className="w-full px-4 py-2 bg-dark-900 border border-dark-700 rounded-lg font-mono text-sm focus:outline-none focus:border-brand-500"
            placeholder="••••••••••••••••"
          />
        </div>
        <button type="submit" className="px-6 py-2 bg-brand-500 hover:bg-brand-400 text-white font-semibold rounded-lg flex items-center gap-2 h-[42px] transition-colors shadow-lg shadow-brand-500/20">
          <Plus size={18} /> Add
        </button>
      </form>

      <div className="flex-1 glass-panel p-6 overflow-y-auto">
        <div className="flex items-center gap-2 mb-4 text-brand-400 font-medium">
          <Key size={18} /> Encrypted Keys
        </div>
        <div className="grid grid-cols-1 xl:grid-cols-2 gap-4">
          {secrets.map((s) => (
            <div key={s.id} className="bg-dark-900 border border-dark-700 p-4 rounded-xl flex items-center justify-between group hover:border-brand-500/50 transition-colors">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-dark-800 flex items-center justify-center border border-dark-700">
                  <Lock size={16} className="text-gray-400" />
                </div>
                <div>
                  <h4 className="font-semibold text-gray-200">{s.name}</h4>
                  {/* Fix #4: Conditional encryption label */}
                  {encryptionAvailable
                    ? <p className="text-xs text-brand-400 font-mono mt-0.5">Encrypted via safeStorage</p>
                    : <p className="text-xs text-yellow-500 font-mono mt-0.5">⚠ Stored as Base64 (not encrypted)</p>
                  }
                </div>
              </div>
              <button
                onClick={() => handleDelete(s.id)}
                className="p-3 bg-red-500/10 text-red-400 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity hover:bg-red-500 hover:text-white"
                title="Delete Secret"
              >
                <Trash2 size={18} />
              </button>
            </div>
          ))}
          {secrets.length === 0 && (
            <div className="col-span-full text-center py-12 text-gray-500">
              Your vault is empty. Add a secret above.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
