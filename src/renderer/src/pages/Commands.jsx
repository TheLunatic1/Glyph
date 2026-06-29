import React, { useState, useEffect } from 'react';
import { Play, Plus, TrendingUp, Trash2 } from 'lucide-react';

export default function Commands() {
  const [commands, setCommands] = useState([]);
  const [newCmdName, setNewCmdName] = useState('');
  const [newCmd, setNewCmd] = useState('');

  useEffect(() => {
    const saved = localStorage.getItem('glyph_commands');
    if (saved) {
      setCommands(JSON.parse(saved));
    } else {
      setCommands([
        { id: 1, name: 'Update System', cmd: 'sudo apt update && sudo apt upgrade -y', uses: 0 },
        { id: 2, name: 'Check Logs', cmd: 'tail -f /var/log/syslog', uses: 0 },
        { id: 3, name: 'List Ports', cmd: 'netstat -tulpn', uses: 0 }
      ]);
    }
  }, []);

  const saveCommands = (cmds) => {
    setCommands(cmds);
    localStorage.setItem('glyph_commands', JSON.stringify(cmds));
  };

  const executeCommand = (id, cmdStr) => {
    window.api.sshShellData(cmdStr + '\n');
    const updated = commands.map(c =>
      c.id === id ? { ...c, uses: (c.uses || 0) + 1 } : c
    );
    saveCommands(updated);
  };

  const deleteCommand = (id, e) => {
    e.stopPropagation();
    saveCommands(commands.filter(c => c.id !== id));
  };

  const addCommand = (e) => {
    e.preventDefault();
    if (!newCmdName || !newCmd) return;
    const newEntry = {
      id: Date.now(),
      name: newCmdName,
      cmd: newCmd,
      uses: 0
    };
    saveCommands([...commands, newEntry]);
    setNewCmdName('');
    setNewCmd('');
  };

  const sortedCommands = [...commands].sort((a, b) => (b.uses || 0) - (a.uses || 0));

  return (
    <div className="p-8 h-full flex flex-col">
      <header className="mb-8">
        <h2 className="text-3xl font-bold text-gray-100">Saved Commands</h2>
        <p className="text-gray-400 mt-2">Manage and execute frequent shell snippets</p>
      </header>

      <form onSubmit={addCommand} className="mb-8 glass-panel p-6 flex gap-4 items-end">
        <div className="flex-1">
          <label className="block text-gray-400 text-sm mb-2">Snippet Name</label>
          <input
            value={newCmdName}
            onChange={e => setNewCmdName(e.target.value)}
            className="w-full px-4 py-2 bg-dark-900 border border-dark-700 rounded-lg focus:outline-none focus:border-brand-500"
            placeholder="e.g. Restart Nginx"
          />
        </div>
        <div className="flex-1">
          <label className="block text-gray-400 text-sm mb-2">Shell Command</label>
          <input
            value={newCmd}
            onChange={e => setNewCmd(e.target.value)}
            className="w-full px-4 py-2 bg-dark-900 border border-dark-700 rounded-lg font-mono text-sm focus:outline-none focus:border-brand-500"
            placeholder="systemctl restart nginx"
          />
        </div>
        <button type="submit" className="px-6 py-2 bg-brand-500 hover:bg-brand-400 text-white font-semibold rounded-lg flex items-center gap-2 h-[42px] transition-colors">
          <Plus size={18} /> Add
        </button>
      </form>

      <div className="flex-1 glass-panel p-6 overflow-y-auto">
        <div className="flex items-center gap-2 mb-4 text-brand-400 font-medium">
          <TrendingUp size={18} /> Sorted by Most Used
        </div>
        <div className="grid grid-cols-1 xl:grid-cols-2 gap-4">
          {sortedCommands.map((c) => (
            <div key={c.id} className="bg-dark-900 border border-dark-700 p-4 rounded-xl flex items-center justify-between group hover:border-brand-500/50 transition-colors">
              <div className="flex-1 min-w-0 mr-3">
                <div className="flex items-center gap-2">
                  <h4 className="font-semibold text-gray-200">{c.name}</h4>
                  {c.uses > 0 && (
                    <span className="text-xs text-dark-400 bg-dark-700 px-2 py-0.5 rounded-full shrink-0">
                      {c.uses} uses
                    </span>
                  )}
                </div>
                <code className="text-brand-400 text-sm mt-1 block truncate">{c.cmd}</code>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                <button
                  onClick={() => executeCommand(c.id, c.cmd)}
                  className="p-3 bg-brand-500/10 text-brand-400 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity hover:bg-brand-500 hover:text-white"
                  title="Execute in Terminal"
                >
                  <Play size={18} fill="currentColor" />
                </button>
                <button
                  onClick={(e) => deleteCommand(c.id, e)}
                  className="p-3 bg-red-500/10 text-red-400 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity hover:bg-red-500 hover:text-white"
                  title="Delete command"
                >
                  <Trash2 size={18} />
                </button>
              </div>
            </div>
          ))}
          {sortedCommands.length === 0 && (
            <div className="col-span-full text-center py-12 text-gray-500">
              No commands saved. Add one above.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
