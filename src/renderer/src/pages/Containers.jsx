import React, { useState, useEffect, useRef } from 'react';
import { Play, Square, RotateCcw, AlertCircle, X, Terminal as TerminalIcon, Network, HardDrive, ChevronDown, ChevronUp } from 'lucide-react';

// ── Animated loading dots ───────────────────────────────────────────────────
function LoadingDots() {
  const [dots, setDots] = useState(0);
  useEffect(() => {
    const t = setInterval(() => setDots(d => (d + 1) % 4), 400);
    return () => clearInterval(t);
  }, []);
  return (
    <span className="inline-flex items-center gap-[3px]">
      {[0, 1, 2].map(i => (
        <span
          key={i}
          className="w-1.5 h-1.5 rounded-full bg-brand-400"
          style={{
            animation: 'dot-bounce 1.2s ease-in-out infinite',
            animationDelay: `${i * 0.2}s`,
          }}
        />
      ))}
    </span>
  );
}

export default function Containers() {
  const [containers, setContainers] = useState([]);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState({});
  const [error, setError] = useState(null);
  const [actionError, setActionError] = useState(null);

  const [activeTab, setActiveTab] = useState('list');
  const [openContainers, setOpenContainers] = useState([]);

  // Compose
  const [composePath, setComposePath] = useState('');
  const [composeLoading, setComposeLoading] = useState(null); // 'up' | 'down' | null
  const [composeOutput, setComposeOutput] = useState('');
  const [showCompose, setShowCompose] = useState(false);

  const fetchContainers = async () => {
    setLoading(true);
    setError(null);
    try {
      const cmd = `docker ps -a --format '{"id":"{{.ID}}", "image":"{{.Image}}", "name":"{{.Names}}", "status":"{{.Status}}", "state":"{{.State}}"}' 2>&1`;
      const output = await window.api.sshExec(cmd);

      if (!output || output.includes('command not found') || output.includes('Cannot connect')) {
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
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchContainers();
  }, []);

  const containerAction = async (id, action) => {
    setActionLoading(prev => ({ ...prev, [id]: action }));
    setActionError(null);
    try {
      await window.api.sshExec(`docker ${action} ${id}`);
      await new Promise(r => setTimeout(r, 800));
      await fetchContainers();
    } catch (err) {
      setActionError(`Failed to ${action} container: ${err?.message || String(err)}`);
    } finally {
      setActionLoading(prev => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
    }
  };

  const handleCompose = async (action) => {
    if (!composePath.trim()) return;
    setComposeLoading(action);
    setComposeOutput('');
    try {
      const flag = action === 'up' ? 'up -d' : 'down';
      const out = await window.api.sshExec(`cd ${composePath.trim()} && docker compose ${flag} 2>&1`);
      setComposeOutput(out || 'Done.');
      await fetchContainers();
    } catch (err) {
      setComposeOutput('Error: ' + (err?.message || String(err)));
    } finally {
      setComposeLoading(null);
    }
  };

  const getStatusColor = (state) =>
    state.toLowerCase() === 'running'
      ? 'text-green-400 bg-green-400/10'
      : 'text-red-400 bg-red-400/10';

  const isRunning = (state) => state?.toLowerCase() === 'running';

  const openContainerTab = (c) => {
    if (!openContainers.find(x => x.id === c.id)) {
      setOpenContainers([...openContainers, c]);
    }
    setActiveTab(c.id);
  };

  const closeContainerTab = (e, id) => {
    e.stopPropagation();
    setOpenContainers(prev => prev.filter(c => c.id !== id));
    if (activeTab === id) setActiveTab('list');
  };

  return (
    <div className="p-8 h-full flex flex-col relative overflow-hidden">
      {/* dot-bounce keyframes injected once */}
      <style>{`
        @keyframes dot-bounce {
          0%, 80%, 100% { transform: scale(0.6); opacity: 0.4; }
          40% { transform: scale(1); opacity: 1; }
        }
      `}</style>

      {/* Top Tab Bar */}
      <div className="flex items-center gap-2 mb-6 border-b border-dark-700 pb-2 overflow-x-auto custom-scrollbar">
        <button
          onClick={() => setActiveTab('list')}
          className={`px-4 py-2 rounded-lg font-medium text-sm transition-colors whitespace-nowrap ${activeTab === 'list' ? 'bg-brand-500 text-white shadow-lg shadow-brand-500/20' : 'text-gray-400 hover:bg-dark-700 hover:text-gray-200'}`}
        >
          Container List
        </button>
        {openContainers.map(c => (
          <div key={c.id} className={`flex items-center gap-2 px-3 py-1.5 rounded-lg font-medium text-sm transition-colors whitespace-nowrap border ${activeTab === c.id ? 'bg-dark-800 border-brand-500/50 text-brand-300' : 'bg-dark-900 border-dark-700 text-gray-400 hover:bg-dark-800'}`}>
            <button onClick={() => setActiveTab(c.id)} className="flex-1 truncate max-w-[150px] text-left">{c.name}</button>
            <button onClick={(e) => closeContainerTab(e, c.id)} className="p-1 hover:text-white hover:bg-dark-700 rounded transition-colors">
              <X size={14} />
            </button>
          </div>
        ))}
      </div>

      {activeTab === 'list' ? (
        <div className="flex flex-col flex-1 overflow-hidden gap-4">
          {/* Header */}
          <header className="flex items-center justify-between shrink-0">
            <div>
              <h2 className="text-3xl font-bold text-gray-100">Docker Containers</h2>
              <p className="text-gray-400 mt-1">Manage running and stopped instances</p>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => setShowCompose(v => !v)}
                className="px-3 py-2.5 bg-dark-700 hover:bg-dark-600 rounded-lg transition-colors flex items-center gap-2 text-gray-200 text-sm font-medium"
              >
                Docker Compose {showCompose ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
              </button>
              <button
                onClick={fetchContainers}
                disabled={loading}
                className="px-3 py-2.5 bg-dark-700 hover:bg-dark-600 rounded-lg transition-colors flex items-center gap-2 text-gray-200 text-sm font-medium disabled:opacity-50"
              >
                <RotateCcw size={18} className={loading ? 'animate-spin' : ''} /> Refresh
              </button>
            </div>
          </header>

          {/* Compose Panel */}
          {showCompose && (
            <div className="shrink-0 bg-dark-900 border border-dark-700 rounded-xl p-4 flex flex-col gap-3">
              <h4 className="text-gray-200 font-semibold text-sm flex items-center gap-2">
                <TerminalIcon size={15} className="text-brand-400" /> Docker Compose
              </h4>
              <div className="flex items-center gap-2">
                <input
                  value={composePath}
                  onChange={e => setComposePath(e.target.value)}
                  placeholder="/path/to/project (where docker-compose.yml is)"
                  className="flex-1 px-3 py-2 bg-dark-800 border border-dark-700 rounded-lg text-gray-200 text-sm focus:outline-none focus:border-brand-500 font-mono"
                />
                <button
                  onClick={() => handleCompose('up')}
                  disabled={!!composeLoading || !composePath.trim()}
                  className="px-4 py-2 bg-green-600 hover:bg-green-500 text-white rounded-lg text-sm font-medium flex items-center gap-2 disabled:opacity-50 transition-colors"
                >
                  {composeLoading === 'up' ? <><LoadingDots /> Running...</> : <><Play size={14} /> Compose Up</>}
                </button>
                <button
                  onClick={() => handleCompose('down')}
                  disabled={!!composeLoading || !composePath.trim()}
                  className="px-4 py-2 bg-red-600 hover:bg-red-500 text-white rounded-lg text-sm font-medium flex items-center gap-2 disabled:opacity-50 transition-colors"
                >
                  {composeLoading === 'down' ? <><LoadingDots /> Running...</> : <><Square size={14} fill="currentColor" /> Compose Down</>}
                </button>
              </div>
              {composeOutput && (
                <pre className="bg-[#0f111a] border border-dark-700 rounded-lg p-3 font-mono text-xs text-gray-300 overflow-y-auto max-h-40 whitespace-pre-wrap break-all custom-scrollbar">
                  {composeOutput}
                </pre>
              )}
            </div>
          )}

          {error && (
            <div className="shrink-0 flex items-center gap-3 p-4 bg-red-500/10 border border-red-500/30 rounded-xl text-red-400">
              <AlertCircle size={20} className="shrink-0" />
              <span className="text-sm">{error}</span>
            </div>
          )}
          {actionError && (
            <div className="shrink-0 flex items-center justify-between gap-3 p-4 bg-orange-500/10 border border-orange-500/30 rounded-xl text-orange-400">
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
              <div className="flex-1 flex items-center justify-center text-gray-400 gap-3">
                <RotateCcw size={20} className="animate-spin" />
                <span>Loading containers</span>
                <LoadingDots />
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
                        <tr key={c.id} onClick={() => openContainerTab(c)} className="border-b border-dark-700/50 hover:bg-dark-700/30 transition-colors cursor-pointer group">
                          <td className="p-4 font-medium text-gray-200 group-hover:text-brand-300 transition-colors">{c.name}</td>
                          <td className="p-4 text-gray-400 font-mono text-sm">{c.image}</td>
                          <td className="p-4">
                            <span className={`px-3 py-1 rounded-full text-xs font-semibold ${getStatusColor(c.state)}`}>
                              {actionLoading[c.id]
                                ? <>{actionLoading[c.id].toUpperCase()}ING <LoadingDots /></>
                                : c.state.toUpperCase()
                              }
                            </span>
                          </td>
                          <td className="p-4 text-gray-400 text-sm">{c.status}</td>
                          <td className="p-4" onClick={(e) => e.stopPropagation()}>
                            <div className="flex items-center justify-end gap-2">
                              {!isRunning(c.state) && (
                                <button disabled={busy} onClick={() => containerAction(c.id, 'start')} title="Start"
                                  className="p-2 rounded-lg bg-green-500/10 text-green-400 hover:bg-green-500 hover:text-white transition-colors disabled:opacity-40">
                                  <Play size={16} fill="currentColor" />
                                </button>
                              )}
                              {isRunning(c.state) && (
                                <button disabled={busy} onClick={() => containerAction(c.id, 'stop')} title="Stop"
                                  className="p-2 rounded-lg bg-red-500/10 text-red-400 hover:bg-red-500 hover:text-white transition-colors disabled:opacity-40">
                                  <Square size={16} fill="currentColor" />
                                </button>
                              )}
                              <button disabled={busy} onClick={() => containerAction(c.id, 'restart')} title="Restart"
                                className="p-2 rounded-lg bg-brand-500/10 text-brand-400 hover:bg-brand-500 hover:text-white transition-colors disabled:opacity-40">
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
      ) : (
        <div className="flex-1 overflow-hidden">
          {openContainers.map(c => c.id === activeTab && (
            <ContainerDetailsView
              key={c.id}
              container={containers.find(x => x.id === c.id) || c}
              onAction={(action) => containerAction(c.id, action)}
              actionLoading={!!actionLoading[c.id]}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ── Container Details ────────────────────────────────────────────────────────
const ContainerDetailsView = ({ container, onAction, actionLoading }) => {
  const [details, setDetails] = useState(null);
  const [logs, setLogs] = useState('');
  const [stats, setStats] = useState(null);
  const [loadingDetails, setLoadingDetails] = useState(true);
  const logsRef = useRef(null);
  const pollRef = useRef(null);

  // Fetch inspect once
  useEffect(() => {
    setLoadingDetails(true);
    window.api.sshExec(`docker inspect ${container.id}`).then(out => {
      try {
        setDetails(JSON.parse(out)[0]);
      } catch (_) {}
      setLoadingDetails(false);
    }).catch(() => setLoadingDetails(false));
  }, [container.id]);

  // Poll logs + stats in PARALLEL for speed
  useEffect(() => {
    let active = true;

    const tick = async () => {
      if (!active) return;
      try {
        const running = container.state?.toLowerCase() === 'running';

        // Run logs and (optionally) stats in parallel
        const promises = [
          window.api.sshExec(`docker logs --tail 200 ${container.id} 2>&1`),
          running
            // Use /proc/meminfo via docker top + exec instead of docker stats (much faster — no sampling delay)
            ? window.api.sshExec(`docker exec ${container.id} cat /proc/meminfo 2>/dev/null || echo ''`)
            : Promise.resolve(null),
        ];

        const [logsOut, memOut] = await Promise.all(promises);

        if (!active) return;

        if (logsOut) setLogs(logsOut);

        if (memOut) {
          // Parse MemTotal and MemAvailable from /proc/meminfo
          const total = parseInt((memOut.match(/MemTotal:\s+(\d+)/) || [])[1] || '0') * 1024;
          const avail = parseInt((memOut.match(/MemAvailable:\s+(\d+)/) || [])[1] || '0') * 1024;
          const used = total - avail;
          const fmt = (b) => b >= 1e9 ? (b / 1e9).toFixed(2) + ' GiB' : (b / 1e6).toFixed(1) + ' MiB';
          setStats({ memUsage: `${fmt(used)} / ${fmt(total)}` });
        } else if (!running) {
          setStats({ memUsage: 'Offline' });
        }
      } catch (_) {}
    };

    tick();
    pollRef.current = setInterval(tick, 3000);
    return () => {
      active = false;
      clearInterval(pollRef.current);
    };
  }, [container.id, container.state]);

  // Auto-scroll logs
  useEffect(() => {
    if (logsRef.current) {
      logsRef.current.scrollTop = logsRef.current.scrollHeight;
    }
  }, [logs]);

  // Derived display values
  const networkMode = details ? Object.keys(details.NetworkSettings?.Networks || {}).join(', ') || 'None' : null;
  const entrypoint = details?.Config?.Entrypoint?.join(' ') || '';
  const cmd = details?.Config?.Cmd?.join(' ') || '';
  const fullCmd = `${entrypoint} ${cmd}`.trim();

  let formattedPorts = null;
  if (details?.NetworkSettings?.Ports) {
    const mapping = [];
    for (const [containerPort, hostBindings] of Object.entries(details.NetworkSettings.Ports)) {
      if (!hostBindings) mapping.push(containerPort);
      else hostBindings.forEach(b => mapping.push(`${b.HostIp || '0.0.0.0'}:${b.HostPort}→${containerPort}`));
    }
    formattedPorts = mapping.length > 0 ? mapping.join(', ') : 'None';
  }

  const isRunning = container.state?.toLowerCase() === 'running';
  const [viewTab, setViewTab] = useState('logs');
  const [updateSettings, setUpdateSettings] = useState({ restart: '', memory: '', cpus: '' });
  const [updateMsg, setUpdateMsg] = useState('');

  const handleUpdate = async (e) => {
    e.preventDefault();
    setUpdateMsg('Updating...');
    try {
      const args = [];
      if (updateSettings.restart) args.push(`--restart=${updateSettings.restart}`);
      if (updateSettings.memory) args.push(`--memory=${updateSettings.memory}`);
      if (updateSettings.cpus) args.push(`--cpus=${updateSettings.cpus}`);
      if (args.length === 0) { setUpdateMsg('No settings changed.'); return; }
      const out = await window.api.sshExec(`docker update ${args.join(' ')} ${container.id}`);
      setUpdateMsg(`Updated: ${out || 'success'}`);
      const inspectOut = await window.api.sshExec(`docker inspect ${container.id}`);
      setDetails(JSON.parse(inspectOut)[0]);
    } catch (err) {
      setUpdateMsg(`Error: ${err.message}`);
    }
  };

  const InfoCard = ({ icon, label, value, wide = false }) => (
    <div className={`bg-dark-900 border border-dark-700 p-4 rounded-xl flex flex-col gap-1 ${wide ? 'col-span-2 lg:col-span-4' : ''}`}>
      <div className="text-gray-500 text-xs font-medium uppercase tracking-wider flex items-center gap-1.5">{icon}{label}</div>
      <div className="text-gray-200 font-mono text-sm break-all">
        {value === null ? <LoadingDots /> : (value || '—')}
      </div>
    </div>
  );

  return (
    <div className="h-full flex flex-col glass-panel overflow-hidden border border-dark-700/50">
      <div className="flex flex-col h-full bg-dark-900/50">
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-dark-800 bg-dark-800/30 shrink-0">
          <div>
            <h3 className="text-xl font-bold text-gray-100 flex items-center gap-3">
              {container.name}
              <span className={`px-2.5 py-0.5 rounded-full text-xs font-semibold ${isRunning ? 'text-green-400 bg-green-400/10' : 'text-red-400 bg-red-400/10'}`}>
                {container.state.toUpperCase()}
              </span>
            </h3>
            <p className="text-gray-400 text-sm font-mono mt-1">@{container.image}</p>
          </div>
          <div className="flex gap-2 justify-end">
            <button disabled={actionLoading || isRunning} onClick={() => onAction('start')}
              className={`px-4 py-2 rounded-lg font-medium text-sm flex items-center gap-2 transition-colors ${!isRunning ? 'bg-green-500 hover:bg-green-400 text-white' : 'bg-dark-800 text-gray-500 cursor-not-allowed'}`}>
              <Play size={16} /> Start
            </button>
            <button disabled={actionLoading || !isRunning} onClick={() => onAction('stop')}
              className={`px-4 py-2 rounded-lg font-medium text-sm flex items-center gap-2 transition-colors ${isRunning ? 'bg-red-500 hover:bg-red-400 text-white' : 'bg-dark-800 text-gray-500 cursor-not-allowed'}`}>
              <Square size={16} /> Stop
            </button>
            <button disabled={actionLoading} onClick={() => onAction('restart')}
              className="px-4 py-2 bg-brand-500 hover:bg-brand-400 text-white rounded-lg font-medium text-sm flex items-center gap-2 transition-colors disabled:opacity-60">
              <RotateCcw size={16} className={actionLoading ? 'animate-spin' : ''} /> Restart
            </button>
          </div>
        </div>

        {/* Inner Tabs */}
        <div className="flex items-center gap-4 px-5 pt-3 border-b border-dark-800 shrink-0">
          {['logs', 'inspect', 'settings'].map(t => (
            <button key={t} onClick={() => setViewTab(t)}
              className={`pb-2 px-1 border-b-2 font-medium text-sm capitalize transition-colors ${viewTab === t ? 'border-brand-500 text-brand-400' : 'border-transparent text-gray-500 hover:text-gray-300'}`}>
              {t}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-5 flex flex-col gap-6 custom-scrollbar">
          {viewTab === 'logs' && (
            <>
              <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 shrink-0">
                <InfoCard icon={<Network size={14} />} label="Network" value={networkMode} />
                <div className="bg-dark-900 border border-dark-700 p-4 rounded-xl flex flex-col gap-1">
                  <div className="text-gray-500 text-xs font-medium uppercase tracking-wider flex items-center gap-1.5"><HardDrive size={14} /> Memory Usage</div>
                  <div className="text-brand-300 font-mono text-sm">
                    {stats ? stats.memUsage : <LoadingDots />}
                  </div>
                </div>
                <InfoCard icon={<Network size={14} />} label="Ports" value={formattedPorts} wide />
                <div className="bg-dark-900 border border-dark-700 p-4 rounded-xl flex flex-col gap-1 col-span-2 lg:col-span-4">
                  <div className="text-gray-500 text-xs font-medium uppercase tracking-wider flex items-center gap-1.5"><TerminalIcon size={14} /> Command</div>
                  <div className="text-green-400 font-mono text-sm break-all">
                    {loadingDetails ? <LoadingDots /> : (fullCmd || '—')}
                  </div>
                </div>
              </div>

              <div className="flex flex-col flex-1 min-h-[300px]">
                <div ref={logsRef} className="flex-1 bg-[#0f111a] rounded-xl border border-dark-700 p-4 font-mono text-xs text-gray-300 overflow-y-auto whitespace-pre-wrap break-all custom-scrollbar select-text">
                  {logs || <span className="text-gray-600 flex items-center gap-2">Waiting for logs <LoadingDots /></span>}
                </div>
              </div>
            </>
          )}

          {viewTab === 'inspect' && (
            <div className="flex flex-col flex-1 min-h-[300px]">
              <div className="flex-1 bg-[#0f111a] rounded-xl border border-dark-700 p-4 font-mono text-xs text-gray-300 overflow-y-auto whitespace-pre-wrap break-all custom-scrollbar select-text">
                {loadingDetails ? <span className="flex items-center gap-2 text-gray-500">Loading inspect data <LoadingDots /></span> : (details ? JSON.stringify(details, null, 2) : 'No data.')}
              </div>
            </div>
          )}

          {viewTab === 'settings' && (
            <div className="flex-1 bg-dark-900 border border-dark-700 rounded-xl p-6">
              <h3 className="text-xl text-gray-100 font-bold mb-4">Update Container Settings</h3>
              <form onSubmit={handleUpdate} className="max-w-md flex flex-col gap-4">
                <div>
                  <label className="block text-gray-400 text-sm mb-1">Restart Policy</label>
                  <select value={updateSettings.restart} onChange={e => setUpdateSettings(p => ({ ...p, restart: e.target.value }))}
                    className="w-full px-3 py-2 bg-dark-800 border border-dark-700 rounded-lg text-gray-200 focus:outline-none focus:border-brand-500">
                    <option value="">Leave unchanged</option>
                    <option value="no">no</option>
                    <option value="on-failure">on-failure</option>
                    <option value="always">always</option>
                    <option value="unless-stopped">unless-stopped</option>
                  </select>
                </div>
                <div>
                  <label className="block text-gray-400 text-sm mb-1">Memory Limit (e.g. 512m, 1g)</label>
                  <input type="text" value={updateSettings.memory} onChange={e => setUpdateSettings(p => ({ ...p, memory: e.target.value }))}
                    placeholder="e.g. 512m" className="w-full px-3 py-2 bg-dark-800 border border-dark-700 rounded-lg text-gray-200 focus:outline-none focus:border-brand-500" />
                </div>
                <div>
                  <label className="block text-gray-400 text-sm mb-1">CPU Limit (e.g. 1.5)</label>
                  <input type="text" value={updateSettings.cpus} onChange={e => setUpdateSettings(p => ({ ...p, cpus: e.target.value }))}
                    placeholder="e.g. 1.5" className="w-full px-3 py-2 bg-dark-800 border border-dark-700 rounded-lg text-gray-200 focus:outline-none focus:border-brand-500" />
                </div>
                <button type="submit" className="px-4 py-2 bg-brand-500 hover:bg-brand-400 text-white rounded-lg font-medium transition-colors mt-2">
                  Apply Updates
                </button>
                {updateMsg && <p className="text-brand-400 text-sm mt-2">{updateMsg}</p>}
              </form>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
