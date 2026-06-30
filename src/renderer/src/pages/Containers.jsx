import React, { useState, useEffect } from 'react';
import { Play, Square, RotateCcw, AlertCircle, X, Terminal as TerminalIcon, Network, Cpu, HardDrive } from 'lucide-react';

export default function Containers() {
  const [containers, setContainers] = useState([]);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState({}); // { [containerId]: 'start'|'stop'|'restart' }
  const [error, setError] = useState(null);
  const [actionError, setActionError] = useState(null); // Fix #9: surface docker action failures
  const [selectedContainer, setSelectedContainer] = useState(null);

  const fetchContainers = async () => {
    setLoading(true);
    setError(null);
    try {
      const cmd = `docker ps -a --format '{"id":"{{.ID}}", "image":"{{.Image}}", "name":"{{.Names}}", "status":"{{.Status}}", "state":"{{.State}}"}' 2>&1`;
      const output = await window.api.sshExec(cmd);

      if (output.includes('command not found') || output.includes('Cannot connect')) {
        setError('Docker is not available on this server.');
        setContainers([]);
        return;
      }

      const lines = output.trim().split('\n').filter(Boolean);
      const parsed = lines.map(line => {
        try { return JSON.parse(line); }
        catch (_) { return null; }
      }).filter(Boolean);

      setContainers(parsed);
    } catch (err) {
      setError('Failed to fetch containers: ' + (err?.message || String(err)));
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchContainers();
  }, []);

  const containerAction = async (id, action) => {
    setActionLoading(prev => ({ ...prev, [id]: action }));
    setActionError(null); // Fix #9: clear previous error
    try {
      await window.api.sshExec(`docker ${action} ${id}`);
      // Wait a moment for Docker to register the state change
      await new Promise(r => setTimeout(r, 800));
      await fetchContainers();
    } catch (err) {
      // Fix #9: Show the error to the user instead of silently swallowing it
      console.error(`docker ${action} failed:`, err);
      setActionError(`Failed to ${action} container: ${err?.message || String(err)}`);
    } finally {
      setActionLoading(prev => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
    }
  };

  const getStatusColor = (state) => {
    return state.toLowerCase() === 'running'
      ? 'text-green-400 bg-green-400/10'
      : 'text-red-400 bg-red-400/10';
  };

  const isRunning = (state) => state?.toLowerCase() === 'running';

  return (
    <div className="p-8 h-full flex flex-col relative">
      {selectedContainer && (
        <ContainerDetailsModal 
          container={selectedContainer} 
          onClose={() => setSelectedContainer(null)} 
          onAction={(action) => containerAction(selectedContainer.id, action)}
          actionLoading={!!actionLoading[selectedContainer.id]}
        />
      )}
      <header className="mb-8 flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold text-gray-100">Docker Containers</h2>
          <p className="text-gray-400 mt-2">Manage running and stopped instances</p>
        </div>
        <button
          onClick={fetchContainers}
          disabled={loading}
          className="p-3 bg-dark-700 hover:bg-dark-600 rounded-lg transition-colors flex items-center gap-2 text-gray-200 disabled:opacity-50"
        >
          <RotateCcw size={18} className={loading ? 'animate-spin' : ''} /> Refresh
        </button>
      </header>

      {error && (
        <div className="mb-6 flex items-center gap-3 p-4 bg-red-500/10 border border-red-500/30 rounded-xl text-red-400">
          <AlertCircle size={20} className="shrink-0" />
          <span className="text-sm">{error}</span>
        </div>
      )}
      {/* Fix #9: Docker action error banner */}
      {actionError && (
        <div className="mb-6 flex items-center justify-between gap-3 p-4 bg-orange-500/10 border border-orange-500/30 rounded-xl text-orange-400">
          <div className="flex items-center gap-3">
            <AlertCircle size={20} className="shrink-0" />
            <span className="text-sm">{actionError}</span>
          </div>
          <button onClick={() => setActionError(null)} className="p-1 hover:text-white transition-colors shrink-0">
            <X size={16} />
          </button>
        </div>
      )}

      <div className="flex-1 glass-panel overflow-hidden flex flex-col">
        {loading ? (
          <div className="flex-1 flex items-center justify-center text-gray-400 gap-2">
            <RotateCcw size={20} className="animate-spin" /> Loading container states...
          </div>
        ) : (
          <div className="overflow-y-auto">
            <table className="w-full text-left">
              <thead className="bg-dark-900 border-b border-dark-700 sticky top-0">
                <tr>
                  <th className="p-4 text-gray-400 font-medium">Name</th>
                  <th className="p-4 text-gray-400 font-medium">Image</th>
                  <th className="p-4 text-gray-400 font-medium">State</th>
                  <th className="p-4 text-gray-400 font-medium">Status</th>
                  <th className="p-4 text-gray-400 font-medium text-right">Actions</th>
                </tr>
              </thead>
              <tbody>
                {containers.map((c) => {
                  const busy = !!actionLoading[c.id];
                  return (
                    <tr key={c.id} onClick={() => setSelectedContainer(c)} className="border-b border-dark-700/50 hover:bg-dark-700/30 transition-colors cursor-pointer group">
                      <td className="p-4 font-medium text-gray-200 group-hover:text-brand-300 transition-colors">{c.name}</td>
                      <td className="p-4 text-gray-400 font-mono text-sm">{c.image}</td>
                      <td className="p-4">
                        <span className={`px-3 py-1 rounded-full text-xs font-semibold ${getStatusColor(c.state)}`}>
                          {actionLoading[c.id]
                            ? actionLoading[c.id].toUpperCase() + 'ING...'
                            : c.state.toUpperCase()
                          }
                        </span>
                      </td>
                      <td className="p-4 text-gray-400 text-sm">{c.status}</td>
                      <td className="p-4" onClick={(e) => e.stopPropagation()}>
                        <div className="flex items-center justify-end gap-2">
                          {/* Start — only when stopped */}
                          {!isRunning(c.state) && (
                            <button
                              disabled={busy}
                              onClick={() => containerAction(c.id, 'start')}
                              title="Start"
                              className="p-2 rounded-lg bg-green-500/10 text-green-400 hover:bg-green-500 hover:text-white transition-colors disabled:opacity-40"
                            >
                              <Play size={16} fill="currentColor" />
                            </button>
                          )}
                          {/* Stop — only when running */}
                          {isRunning(c.state) && (
                            <button
                              disabled={busy}
                              onClick={() => containerAction(c.id, 'stop')}
                              title="Stop"
                              className="p-2 rounded-lg bg-red-500/10 text-red-400 hover:bg-red-500 hover:text-white transition-colors disabled:opacity-40"
                            >
                              <Square size={16} fill="currentColor" />
                            </button>
                          )}
                          {/* Restart — always available */}
                          <button
                            disabled={busy}
                            onClick={() => containerAction(c.id, 'restart')}
                            title="Restart"
                            className="p-2 rounded-lg bg-brand-500/10 text-brand-400 hover:bg-brand-500 hover:text-white transition-colors disabled:opacity-40"
                          >
                            <RotateCcw size={16} className={actionLoading[c.id] === 'restart' ? 'animate-spin' : ''} />
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
            {containers.length === 0 && !error && (
              <div className="text-center p-12 text-gray-500">No Docker containers found.</div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

const ContainerDetailsModal = ({ container, onClose, onAction, actionLoading }) => {
  const [details, setDetails] = useState(null);
  const [logs, setLogs] = useState('');
  const [stats, setStats] = useState(null);
  const logsEndRef = React.useRef(null);

  // fetch inspect data once
  useEffect(() => {
    const fetchDetails = async () => {
      try {
        const inspectOut = await window.api.sshExec(`docker inspect ${container.id}`);
        const parsed = JSON.parse(inspectOut)[0];
        setDetails(parsed);
      } catch (e) {
        console.error(e);
      }
    };
    fetchDetails();
  }, [container.id]);

  useEffect(() => {
    let active = true;

    // Fix #6: Use setInterval instead of recursive setTimeout to avoid timer leak race condition
    const tick = async () => {
      if (!active) return;
      try {
        // Logs
        const logsOut = await window.api.sshExec(`docker logs --tail 100 ${container.id} 2>&1`);
        if (active && logsOut) setLogs(logsOut);

        // Stats
        if (container.state.toLowerCase() === 'running') {
          const statsOut = await window.api.sshExec(`docker stats --no-stream --format '{"memUsage":"{{.MemUsage}}"}' ${container.id}`);
          if (active && statsOut) setStats(JSON.parse(statsOut.trim()));
        } else {
          setStats({ memUsage: 'Offline' });
        }
      } catch (e) {}
    };

    tick(); // run immediately on mount
    const timer = setInterval(tick, 2000);
    return () => { active = false; clearInterval(timer); };
  }, [container.id, container.state]);

  // scroll logs
  useEffect(() => {
    if (logsEndRef.current) {
      logsEndRef.current.scrollTop = logsEndRef.current.scrollHeight;
    }
  }, [logs]);

  // Extract nicely formatted details
  const networkMode = details ? Object.keys(details.NetworkSettings.Networks || {}).join(', ') : '...';
  const entrypoint = details?.Config?.Entrypoint ? details.Config.Entrypoint.join(' ') : '';
  const cmd = details?.Config?.Cmd ? details.Config.Cmd.join(' ') : '';
  const fullCmd = `${entrypoint} ${cmd}`.trim();
  
  // Format ports
  let formattedPorts = '...';
  if (details && details.NetworkSettings.Ports) {
    const p = details.NetworkSettings.Ports;
    const mapping = [];
    for (const [containerPort, hostBindings] of Object.entries(p)) {
      if (!hostBindings) {
        mapping.push(containerPort);
      } else {
        hostBindings.forEach(b => {
          mapping.push(`${b.HostIp || '0.0.0.0'}:${b.HostPort}->${containerPort}`);
        });
      }
    }
    formattedPorts = mapping.length > 0 ? mapping.join(', ') : 'None';
  }

  const isRunning = container.state?.toLowerCase() === 'running';

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-200" onClick={onClose}>
      <div className="bg-dark-900 border border-dark-700 w-full max-w-2xl rounded-2xl shadow-2xl flex flex-col max-h-[90vh]" onClick={e => e.stopPropagation()}>
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-dark-800 bg-dark-800/30">
          <div>
            <h3 className="text-xl font-bold text-gray-100 flex items-center gap-3">
              {container.name}
              <span className={`px-2.5 py-0.5 rounded-full text-xs font-semibold ${isRunning ? 'text-green-400 bg-green-400/10' : 'text-red-400 bg-red-400/10'}`}>
                {container.state.toUpperCase()}
              </span>
            </h3>
            <p className="text-gray-400 text-sm font-mono mt-1">@{container.image}</p>
          </div>
          <button onClick={onClose} className="p-2 text-gray-500 hover:text-gray-200 transition-colors bg-dark-800 hover:bg-dark-700 rounded-lg">
            <X size={20}/>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-5 flex flex-col gap-6 custom-scrollbar">
          {/* Quick Actions */}
          <div className="flex gap-2 justify-end">
             <button disabled={actionLoading} onClick={() => onAction('start')} className={`px-4 py-2 rounded-lg font-medium text-sm flex items-center gap-2 transition-colors ${!isRunning ? 'bg-green-500 hover:bg-green-400 text-white' : 'bg-dark-800 text-gray-500 cursor-not-allowed'}`}>
               <Play size={16} /> Start
             </button>
             <button disabled={actionLoading} onClick={() => onAction('stop')} className={`px-4 py-2 rounded-lg font-medium text-sm flex items-center gap-2 transition-colors ${isRunning ? 'bg-red-500 hover:bg-red-400 text-white' : 'bg-dark-800 text-gray-500 cursor-not-allowed'}`}>
               <Square size={16} /> Stop
             </button>
             <button disabled={actionLoading} onClick={() => onAction('restart')} className="px-4 py-2 bg-brand-500 hover:bg-brand-400 text-white rounded-lg font-medium text-sm flex items-center gap-2 transition-colors">
               <RotateCcw size={16} className={actionLoading ? 'animate-spin' : ''} /> Restart
             </button>
          </div>

          {/* Details Grid */}
          <div className="grid grid-cols-2 gap-4">
             <div className="glass-panel p-4 flex flex-col gap-1">
               <div className="text-gray-500 text-xs font-medium uppercase tracking-wider flex items-center gap-1.5"><Network size={14}/> Network</div>
               <div className="text-gray-200 font-mono text-sm truncate" title={networkMode}>{networkMode}</div>
             </div>
             <div className="glass-panel p-4 flex flex-col gap-1">
               <div className="text-gray-500 text-xs font-medium uppercase tracking-wider flex items-center gap-1.5"><HardDrive size={14}/> Memory Usage</div>
               <div className="text-brand-300 font-mono text-sm">{stats ? stats.memUsage : '...'}</div>
             </div>
             <div className="glass-panel p-4 flex flex-col gap-1 col-span-2">
               <div className="text-gray-500 text-xs font-medium uppercase tracking-wider flex items-center gap-1.5"><Network size={14}/> Ports</div>
               <div className="text-gray-200 font-mono text-sm">{formattedPorts}</div>
             </div>
             <div className="glass-panel p-4 flex flex-col gap-1 col-span-2">
               <div className="text-gray-500 text-xs font-medium uppercase tracking-wider flex items-center gap-1.5"><TerminalIcon size={14}/> Command</div>
               <div className="text-green-400 font-mono text-sm break-all">{fullCmd || '...'}</div>
             </div>
             <div className="glass-panel p-4 flex flex-col gap-1 col-span-2">
               <div className="text-gray-500 text-xs font-medium uppercase tracking-wider flex items-center gap-1.5"><Cpu size={14}/> Container ID</div>
               <div className="text-gray-400 font-mono text-sm">{details ? details.Id : container.id}</div>
             </div>
          </div>

          {/* Logs */}
          <div className="flex flex-col flex-1 min-h-[300px]">
            <h4 className="text-gray-400 font-medium mb-2 flex items-center gap-2"><TerminalIcon size={16}/> Live Logs (tail 100)</h4>
            <div ref={logsEndRef} className="flex-1 bg-[#0f111a] rounded-xl border border-dark-700 p-4 font-mono text-xs text-gray-300 overflow-y-auto whitespace-pre-wrap break-all custom-scrollbar select-text">
              {logs || <span className="text-gray-600 animate-pulse">Waiting for logs...</span>}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
