import React, { useState, useEffect } from 'react';
import { Lock, Plus, Trash2, Key } from 'lucide-react';

export default function Secrets() {
  const [secrets, setSecrets] = useState([]);
  const [name, setName] = useState('');
  const [value, setValue] = useState('');

  const loadSecrets = async () => {
    const list = await window.api.getSecrets();
    setSecrets(list);
  };

  useEffect(() => {
    loadSecrets();
  }, []);

  const handleAdd = async (e) => {
    e.preventDefault();
    if (!name || !value) return;
    await window.api.addSecret(name, value);
    setName('');
    setValue('');
    loadSecrets();
  };

  const handleDelete = async (id) => {
    await window.api.deleteSecret(id);
    loadSecrets();
  };

  return (
    <div className="p-8 h-full flex flex-col">
      <header className="mb-8">
        <h2 className="text-3xl font-bold text-gray-100 flex items-center gap-3">
          <Lock className="text-brand-400" size={28} /> Secrets Vault
        </h2>
        <p className="text-gray-400 mt-2">
          Store highly sensitive tokens (e.g. GitHub API keys) encrypted on disk. These can be securely injected into any active terminal session without exposing the plaintext.
        </p>
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
                  <p className="text-xs text-brand-400 font-mono mt-0.5">Encrypted via safeStorage</p>
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
