import React, { useEffect, useState } from 'react';
import { X, Cpu, MemoryStick, HardDrive, Wifi, ArrowDown, ArrowUp, CircuitBoard, Info } from 'lucide-react';
import OsLogo from '../components/OsLogo';

// ── Helpers ──────────────────────────────────────────────────────────────────
const fmtBytes = (bytes) => {
  const n = parseFloat(bytes) || 0;
  if (n >= 1e9) return (n / 1e9).toFixed(2) + ' GB';
  if (n >= 1e6) return (n / 1e6).toFixed(1) + ' MB';
  if (n >= 1e3) return (n / 1e3).toFixed(1) + ' KB';
  return n.toFixed(0) + ' B';
};

const fmtSpeed = (bps) => {
  const n = parseFloat(bps) || 0;
  if (n >= 1e6) return (n / 1e6).toFixed(1) + ' MB/s';
  if (n >= 1e3) return (n / 1e3).toFixed(1) + ' KB/s';
  return n.toFixed(0) + ' B/s';
};

const parseNet = (raw) => {
  if (!raw) return [];
  return raw.trim().split('\n').map(line => {
    const m = line.match(/^(\S+)\s+RX:(\d+)\s+TX:(\d+)\s+RXS:(\d+)\s+TXS:(\d+)/);
    if (!m) return null;
    return { iface: m[1], rx: m[2], tx: m[3], rxs: m[4], txs: m[5] };
  }).filter(Boolean);
};

// ── Bar helper ────────────────────────────────────────────────────────────────
// inverted=true: high value is GOOD (e.g. Free memory, Swap Free)
const Bar = ({ label, value, max, unit = '', inverted = false }) => {
  const pct = max > 0 ? Math.min(100, (value / max) * 100) : 0;
  const score = inverted ? (100 - pct) : pct; // for coloring: low score = good
  const col = score > 90 ? 'bg-red-500' : score > 75 ? 'bg-yellow-500' : 'bg-brand-500';
  return (
    <div className="mb-3">
      <div className="flex justify-between text-xs text-gray-400 mb-1">
        <span>{label}</span>
        <span className="font-mono text-gray-200">{value.toFixed(0)}{unit} / {max.toFixed(0)}{unit}</span>
      </div>
      <div className="w-full h-2 bg-dark-700 rounded-full overflow-hidden">
        <div className={`h-2 rounded-full ${col} transition-all duration-700`} style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
};

// ── Modal shell ───────────────────────────────────────────────────────────────
function ModalShell({ title, icon, onClose, children }) {
  return (
    <div className="fixed inset-0 z-50 bg-dark-900/80 backdrop-blur-sm flex items-center justify-center p-4" onClick={onClose}>
      <div className="glass-panel w-full max-w-2xl border border-brand-500/30 overflow-hidden flex flex-col shadow-2xl" onClick={e => e.stopPropagation()}>
        <div className="flex items-center justify-between p-4 border-b border-dark-700 bg-dark-800/50">
          <h3 className="font-semibold text-gray-100 text-lg flex items-center gap-2">{icon}{title}</h3>
          <button onClick={onClose} className="p-1 hover:bg-dark-700 rounded-md text-gray-400 hover:text-white">
            <X size={20} />
          </button>
        </div>
        <div className="p-6 overflow-auto max-h-[75vh]">{children}</div>
      </div>
    </div>
  );
}

// ── CPU Modal ─────────────────────────────────────────────────────────────────
function CpuModal({ raw, cores, temp, onClose }) {
  const lines = (raw || '').split('\n');
  const loadMatch = lines[0] && lines[0].match(/load average:\s*([\d.]+),\s*([\d.]+),\s*([\d.]+)/);
  const tasks = lines.find(l => l.match(/Tasks/i)) || '';

  // Parse per-core data: "cpu0 12.5"
  const coreRows = (cores || '').trim().split('\n').map(line => {
    const m = line.trim().match(/^(cpu\d+)\s+([\d.]+)/);
    return m ? { id: m[1], pct: parseFloat(m[2]) } : null;
  }).filter(Boolean);

  // Parse temperature lines from sensors or /sys/class/thermal
  const tempRows = (temp || '').trim().split('\n').filter(Boolean);

  return (
    <ModalShell title="CPU Details" icon={<Cpu size={20} className="text-brand-400" />} onClose={onClose}>
      {loadMatch && (
        <div className="mb-5">
          <p className="text-gray-400 text-xs uppercase tracking-wider mb-2">Load Average</p>
          <div className="grid grid-cols-3 gap-3">
            {['1 min', '5 min', '15 min'].map((l, i) => (
              <div key={l} className="bg-dark-900 rounded-xl p-3 text-center border border-dark-700">
                <p className="text-2xl font-bold text-brand-400">{loadMatch[i + 1]}</p>
                <p className="text-gray-500 text-xs mt-1">{l}</p>
              </div>
            ))}
          </div>
        </div>
      )}
      {tasks && <div className="mb-5 text-xs text-gray-300 bg-dark-900 rounded-xl p-3 font-mono border border-dark-700">{tasks.trim()}</div>}

      {coreRows.length > 0 && (
        <div className="mb-5">
          <p className="text-gray-400 text-xs uppercase tracking-wider mb-3">Per-Thread Usage ({coreRows.length} threads)</p>
          <div className="grid grid-cols-2 gap-2">
            {coreRows.map(c => {
              const col = c.pct > 90 ? 'bg-red-500' : c.pct > 60 ? 'bg-yellow-500' : 'bg-brand-500';
              return (
                <div key={c.id} className="bg-dark-900 rounded-lg px-3 py-2 border border-dark-700">
                  <div className="flex justify-between text-xs mb-1">
                    <span className="text-gray-400 font-mono">{c.id}</span>
                    <span className={`font-mono font-bold ${c.pct > 90 ? 'text-red-400' : c.pct > 60 ? 'text-yellow-400' : 'text-brand-400'}`}>{c.pct.toFixed(1)}%</span>
                  </div>
                  <div className="w-full h-1.5 bg-dark-700 rounded-full overflow-hidden">
                    <div className={`h-1.5 rounded-full ${col} transition-all duration-700`} style={{ width: `${c.pct}%` }} />
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {tempRows.length > 0 ? (
        <div>
          <p className="text-gray-400 text-xs uppercase tracking-wider mb-3">Temperature</p>
          <div className="grid grid-cols-2 gap-2">
            {tempRows.map((line, i) => {
              // Match patterns: "+45.0°C", "45.0°C", "45C", "45.0 C"
              const tempMatch = line.match(/([+\-]?\d+(?:\.\d+)?)\s*°?\s*C(?:\s|$)/i);
              const tempVal = tempMatch ? parseFloat(tempMatch[1]) : 0;
              const hot = tempVal > 80;
              const warm = tempVal > 65;
              // Label: everything before the number
              const label = line.replace(/[+\-]?\d+(?:\.\d+)?\s*°?\s*C.*/i, '').replace(/[:\s]+$/, '').trim() || `Zone ${i}`;
              return (
                <div key={i} className={`bg-dark-900 rounded-lg px-3 py-2 border ${hot ? 'border-red-500/40' : warm ? 'border-yellow-500/40' : 'border-dark-700'}`}>
                  <p className="text-gray-400 text-xs truncate">{label}</p>
                  <p className={`font-mono font-bold mt-0.5 ${hot ? 'text-red-400' : warm ? 'text-yellow-400' : 'text-green-400'}`}>
                    {tempVal > 0 ? `${tempVal.toFixed(0)}°C` : '—'}
                  </p>
                </div>
              );
            })}
          </div>
        </div>
      ) : (
        <div className="mb-5">
          <p className="text-gray-400 text-xs uppercase tracking-wider mb-3">Temperature</p>
          <div className="bg-dark-900 rounded-xl p-4 border border-dark-700">
            <p className="text-gray-400 text-sm mb-3">No thermal sensors detected. If this is a physical server, you may need to install the sensors package:</p>
            <div className="bg-dark-800/50 rounded-lg p-3 border border-dark-700">
              <code className="block bg-dark-900 text-brand-300 p-2 rounded font-mono text-sm border border-dark-600">sudo apt install lm-sensors</code>
              <p className="text-xs text-gray-500 mt-2">Note: Virtual Machines (VPS) and Shared Hosting environments rarely expose physical hardware sensors to the guest OS.</p>
            </div>
          </div>
        </div>
      )}
      {!cores && !raw && <p className="text-gray-500 text-center py-8">Waiting for data...</p>}
    </ModalShell>
  );
}

// ── Memory Modal ──────────────────────────────────────────────────────────────
function MemModal({ raw, onClose }) {
  const lines = (raw || '').split('\n').filter(Boolean);
  let memTotal = 0, memUsed = 0, memFree = 0, memBuff = 0;
  let swapTotal = 0, swapUsed = 0, swapFree = 0;
  const memLine = lines.find(l => /^Mem/i.test(l));
  const swapLine = lines.find(l => /^Swap/i.test(l));
  if (memLine) { const p = memLine.trim().split(/\s+/); memTotal = +p[1]; memUsed = +p[2]; memFree = +p[3]; memBuff = +p[5] || 0; }
  if (swapLine) { const p = swapLine.trim().split(/\s+/); swapTotal = +p[1]; swapUsed = +p[2]; swapFree = +p[3]; }
  const humanMB = v => v >= 1024 ? (v / 1024).toFixed(1) + ' GB' : v + ' MB';
  return (
    <ModalShell title="Memory Details" icon={<MemoryStick size={20} className="text-purple-400" />} onClose={onClose}>
      {memTotal > 0 ? (
        <>
          <div className="grid grid-cols-3 gap-3 mb-6">
            {[{ l: 'Total', v: memTotal }, { l: 'Used', v: memUsed }, { l: 'Free', v: memFree }].map(i => (
              <div key={i.l} className="bg-dark-900 rounded-xl p-3 text-center border border-dark-700">
                <p className="text-xl font-bold text-gray-100">{humanMB(i.v)}</p>
                <p className="text-gray-500 text-xs mt-1">{i.l}</p>
              </div>
            ))}
          </div>
          <p className="text-gray-400 text-xs uppercase tracking-wider mb-3">RAM Breakdown</p>
          <Bar label="Used" value={memUsed} max={memTotal} unit=" MB" />
          <Bar label="Buff/Cache" value={memBuff} max={memTotal} unit=" MB" />
          <Bar label="Free" value={memFree} max={memTotal} unit=" MB" inverted={true} />
          {swapTotal > 0 && (<>
            <p className="text-gray-400 text-xs uppercase tracking-wider mt-5 mb-3">Swap</p>
            <Bar label="Swap Used" value={swapUsed} max={swapTotal} unit=" MB" />
            <Bar label="Swap Free" value={swapFree} max={swapTotal} unit=" MB" inverted={true} />
          </>)}
        </>
      ) : <p className="text-gray-500 text-center py-8">Waiting for data...</p>}
    </ModalShell>
  );
}

// ── Disk Modal ────────────────────────────────────────────────────────────────
function DiskModal({ raw, onClose }) {
  const rows = (raw || '').split('\n').filter(Boolean).slice(1).map(l => l.trim().split(/\s+/));
  return (
    <ModalShell title="Disk Usage" icon={<HardDrive size={20} className="text-orange-400" />} onClose={onClose}>
      {rows.length > 0 ? rows.map((r, i) => {
        const pct = parseFloat((r[4] || '0').replace('%', ''));
        return (
          <div key={i} className="mb-4 bg-dark-900 rounded-xl p-4 border border-dark-700">
            <div className="flex justify-between items-center mb-2">
              <span className="font-mono text-gray-200 text-sm font-semibold">{r[5]}</span>
              <span className="text-xs text-gray-500">{r[0]}</span>
            </div>
            <div className="w-full h-3 bg-dark-700 rounded-full overflow-hidden mb-2">
              <div className={`h-3 rounded-full transition-all duration-700 ${pct > 90 ? 'bg-red-500' : pct > 75 ? 'bg-yellow-500' : 'bg-brand-500'}`} style={{ width: `${pct}%` }} />
            </div>
            <div className="flex justify-between text-xs text-gray-400">
              <span>Used: <span className="text-gray-200 font-mono">{r[2]}</span></span>
              <span>Free: <span className="text-gray-200 font-mono">{r[3]}</span></span>
              <span>Total: <span className="text-gray-200 font-mono">{r[1]}</span></span>
              <span className={`font-bold ${pct > 90 ? 'text-red-400' : pct > 75 ? 'text-yellow-400' : 'text-brand-400'}`}>{pct}%</span>
            </div>
          </div>
        );
      }) : <p className="text-gray-500 text-center py-8">Waiting for data...</p>}
    </ModalShell>
  );
}

// ── Network Modal ─────────────────────────────────────────────────────────────
function NetModal({ raw, onClose }) {
  const rows = parseNet(raw);
  return (
    <ModalShell title="Network Interfaces" icon={<Wifi size={20} className="text-cyan-400" />} onClose={onClose}>
      {rows.length > 0 ? (
        <div className="space-y-3">
          <div className="grid grid-cols-5 gap-2 text-xs text-gray-500 uppercase tracking-wider px-3 pb-2 border-b border-dark-700">
            <span className="col-span-1">Interface</span>
            <span className="col-span-1 text-center text-green-400">↓ Speed</span>
            <span className="col-span-1 text-center text-blue-400">↑ Speed</span>
            <span className="col-span-1 text-center text-gray-500">Total RX</span>
            <span className="col-span-1 text-center text-gray-500">Total TX</span>
          </div>
          {rows.map(r => (
            <div key={r.iface} className="grid grid-cols-5 gap-2 items-center bg-dark-900 rounded-xl px-3 py-3 border border-dark-700">
              <div className="col-span-1 flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-green-400 shrink-0"></div>
                <span className="font-mono font-semibold text-gray-100 text-sm truncate">{r.iface}</span>
              </div>
              <div className="col-span-1 text-center">
                <span className="text-green-400 font-mono font-bold text-sm">{fmtSpeed(r.rxs)}</span>
              </div>
              <div className="col-span-1 text-center">
                <span className="text-blue-400 font-mono font-bold text-sm">{fmtSpeed(r.txs)}</span>
              </div>
              <div className="col-span-1 text-center">
                <span className="text-gray-400 font-mono text-xs">{fmtBytes(r.rx)}</span>
              </div>
              <div className="col-span-1 text-center">
                <span className="text-gray-400 font-mono text-xs">{fmtBytes(r.tx)}</span>
              </div>
            </div>
          ))}
          <p className="text-gray-600 text-xs pt-2 text-center">Speed updates every 3 seconds</p>
        </div>
      ) : <p className="text-gray-500 text-center py-8">Waiting for network data...</p>}
    </ModalShell>
  );
}

// ── GPU Modal ─────────────────────────────────────────────────────────────────
function GpuModal({ raw, onClose }) {
  if (raw === 'NO_GPU') {
    return (
      <ModalShell title="GPU Details" icon={<CircuitBoard size={20} className="text-emerald-400" />} onClose={onClose}>
        <div className="bg-dark-900 rounded-xl p-6 border border-dark-700">
          <h4 className="text-gray-100 font-bold text-lg mb-2">No GPU Stats Available</h4>
          <p className="text-gray-400 text-sm mb-6">We could not detect any GPU monitoring tools on this server. If you have a GPU installed, you may need to install the appropriate drivers and tools:</p>
          
          <div className="space-y-4">
            <div className="bg-dark-800/50 rounded-lg p-4 border border-dark-700">
              <h5 className="font-semibold text-green-400 mb-2">NVIDIA GPUs</h5>
              <code className="block bg-dark-900 text-brand-300 p-3 rounded font-mono text-sm border border-dark-600">sudo apt install nvidia-utils-535</code>
              <p className="text-xs text-gray-500 mt-2">Glyph uses <code className="text-gray-400">nvidia-smi</code> to fetch metrics.</p>
            </div>
            
            <div className="bg-dark-800/50 rounded-lg p-4 border border-dark-700">
              <h5 className="font-semibold text-red-400 mb-2">AMD GPUs</h5>
              <code className="block bg-dark-900 text-brand-300 p-3 rounded font-mono text-sm border border-dark-600">sudo apt install rocm-smi</code>
              <p className="text-xs text-gray-500 mt-2">Glyph will also try to read <code className="text-gray-400">/sys/class/drm</code> directly if ROCm is unavailable.</p>
            </div>
            
            <div className="bg-dark-800/50 rounded-lg p-4 border border-dark-700">
              <h5 className="font-semibold text-blue-400 mb-2">Intel GPUs</h5>
              <code className="block bg-dark-900 text-brand-300 p-3 rounded font-mono text-sm border border-dark-600">sudo apt install intel-gpu-tools</code>
              <p className="text-xs text-gray-500 mt-2">Provides the <code className="text-gray-400">intel_gpu_top</code> command.</p>
            </div>
          </div>
        </div>
      </ModalShell>
    );
  }

  // Format: index, name, util %, temp, memTotal, memUsed
  const rows = (raw || '').split('\n').filter(Boolean).map(l => l.split(', '));
  return (
    <ModalShell title="GPU Details" icon={<CircuitBoard size={20} className="text-emerald-400" />} onClose={onClose}>
      {rows.length > 0 ? (
        <div className="space-y-4">
          {rows.map(r => {
            if (r.length < 6) return null;
            const [id, name, util, temp, memTotalStr, memUsedStr] = r;
            const pct = parseFloat(util) || 0;
            const memTotal = parseFloat(memTotalStr) || 0;
            const memUsed = parseFloat(memUsedStr) || 0;
            const tempVal = parseFloat(temp) || 0;
            return (
              <div key={id} className="bg-dark-900 rounded-xl p-5 border border-dark-700">
                <div className="flex justify-between items-center mb-4">
                  <h4 className="text-gray-100 font-bold text-lg">{name}</h4>
                  <span className="text-gray-500 text-sm font-mono">GPU {id}</span>
                </div>
                
                <div className="grid grid-cols-2 gap-4 mb-4">
                  <div className="bg-dark-800/50 rounded-lg p-3 border border-dark-700">
                    <p className="text-gray-400 text-xs uppercase mb-1">Temperature</p>
                    <p className={`font-mono font-bold text-xl ${tempVal > 80 ? 'text-red-400' : tempVal > 65 ? 'text-yellow-400' : 'text-green-400'}`}>
                      {tempVal > 0 ? `${tempVal}°C` : '—'}
                    </p>
                  </div>
                  <div className="bg-dark-800/50 rounded-lg p-3 border border-dark-700">
                    <p className="text-gray-400 text-xs uppercase mb-1">Core Usage</p>
                    <p className={`font-mono font-bold text-xl ${pct > 90 ? 'text-red-400' : pct > 70 ? 'text-yellow-400' : 'text-brand-400'}`}>
                      {pct}%
                    </p>
                  </div>
                </div>

                <Bar label="VRAM Usage" value={memUsed} max={memTotal} unit=" MB" />
              </div>
            );
          })}
        </div>
      ) : <p className="text-gray-500 text-center py-8">No NVIDIA GPUs detected or driver not installed.</p>}
    </ModalShell>
  );
}

// ── Circular Ring ─────────────────────────────────────────────────────────────
const CircularProgress = ({ percentage, label, onClick }) => {
  const isNa = percentage < 0;
  const dispPct = isNa ? 0 : percentage;
  const radius = 28;
  const circumference = 2 * Math.PI * radius;
  const strokeDashoffset = circumference - (dispPct / 100) * circumference;
  const color = isNa ? '#3f3f46' : percentage > 90 ? '#ef4444' : percentage > 70 ? '#f59e0b' : '#6366f1';
  return (
    <div onClick={onClick} className={`flex flex-col items-center justify-center py-5 px-2 glass-panel cursor-pointer hover:border-brand-500/50 transition-colors group ${isNa ? 'opacity-50 hover:opacity-100' : ''}`}>
      <div className="relative w-20 h-20 flex items-center justify-center group-hover:scale-105 transition-transform">
        <svg className="transform -rotate-90 w-20 h-20">
          <circle cx="40" cy="40" r={radius} className="stroke-dark-700" strokeWidth="6" fill="transparent" />
          <circle cx="40" cy="40" r={radius} stroke={color} strokeWidth="6" fill="transparent"
            strokeDasharray={circumference} strokeDashoffset={strokeDashoffset}
            strokeLinecap="round" style={{ transition: 'stroke-dashoffset 1s ease' }} />
        </svg>
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <span className="text-xl font-bold">{isNa ? 'N/A' : `${Math.round(percentage)}%`}</span>
        </div>
      </div>
      <span className="mt-3 text-gray-400 font-medium tracking-wide uppercase text-xs">{label}</span>
    </div>
  );
};

// Interfaces to exclude from the network aggregate total
// (loopback, Docker bridges, veth pairs, virtual adaptors)
const VIRTUAL_IFACE_RE = /^(lo|docker\d*|br-|veth|virbr|dummy|bond|ifb|nlmon|teql|tun\d*|tap\d*)/;

// ── Network Speed Card ────────────────────────────────────────────────────────
const NetworkCard = ({ raw, onClick }) => {
  const rows = parseNet(raw);
  const physRows = rows.filter(r => !VIRTUAL_IFACE_RE.test(r.iface));
  // Fall back to all interfaces if physical filtering leaves nothing
  const activeRows = physRows.length > 0 ? physRows : rows;
  const totRxs = activeRows.reduce((s, r) => s + parseFloat(r.rxs), 0);
  const totTxs = activeRows.reduce((s, r) => s + parseFloat(r.txs), 0);

  return (
    <div onClick={onClick} className="flex flex-col items-center justify-center py-5 px-2 glass-panel cursor-pointer hover:border-brand-500/50 transition-colors group">
      <div className="h-20 flex flex-col items-center justify-center gap-1.5 group-hover:scale-105 transition-transform whitespace-nowrap">
        <div className="flex items-center justify-center gap-1.5">
          <ArrowDown size={14} className="text-green-400 shrink-0" />
          <span className="font-mono font-bold text-lg text-green-400 leading-none">{fmtSpeed(totRxs)}</span>
        </div>
        <div className="flex items-center justify-center gap-1.5">
          <ArrowUp size={14} className="text-blue-400 shrink-0" />
          <span className="font-mono font-bold text-lg text-blue-400 leading-none">{fmtSpeed(totTxs)}</span>
        </div>
      </div>
      <span className="mt-3 text-gray-400 font-medium tracking-wide uppercase text-xs group-hover:text-gray-200 transition-colors">Network</span>
    </div>
  );
};

// ── Dashboard ─────────────────────────────────────────────────────────────────
export default function Dashboard({ server }) {
  const [stats, setStats] = useState({ cpu: 0, mem: 0, disk: 0, gpu: null, uptime: 'Loading...', users: '0' });
  const [rawStats, setRawStats] = useState({ top: '', free: '', df: '', net: '', cores: '', temp: '', gpu: '' });
  const [activeModal, setActiveModal] = useState(null);

  useEffect(() => {
    const remove = window.api.onSshStats((data) => {
      let cpu = 0, mem = 0, disk = 0, gpuAvg = null;
      try {
        // Prefer accurate /proc/stat aggregate; fall back to top 'us' field
        if (data.cpuPct !== undefined && data.cpuPct >= 0) {
          cpu = data.cpuPct;
        } else if (data.top) {
          const m = data.top.match(/([\d.]+)\s*us/);
          if (m) cpu = parseFloat(m[1]);
        }
        if (data.free) {
          const memLine = data.free.split('\n').find(l => /^Mem/i.test(l));
          if (memLine) { const p = memLine.trim().split(/\s+/); mem = (+p[2] / +p[1]) * 100; }
        }
        if (data.df) {
          const lines = data.df.split('\n').filter(Boolean);
          if (lines[1]) { const p = lines[1].trim().split(/\s+/); disk = parseFloat((p[4] || '0').replace('%', '')); }
        }
        
        if (data.gpu) {
          if (data.gpu.trim() === 'NO_GPU') {
            gpuAvg = -1;
          } else {
            let total = 0, count = 0;
            data.gpu.split('\n').filter(Boolean).forEach(l => {
               const parts = l.split(', ');
               if (parts.length >= 3) { total += parseFloat(parts[2] || '0'); count++; }
            });
            if (count > 0) gpuAvg = total / count;
          }
        }

      } catch (e) {}
      setStats(prev => ({
        cpu: cpu >= 0 ? cpu : prev.cpu,
        mem: mem || prev.mem,
        disk: disk || prev.disk,
        gpu: gpuAvg !== null ? gpuAvg : prev.gpu,
        uptime: data.uptime ? data.uptime.trim() : prev.uptime,
        users: data.users ? data.users.trim() : prev.users,
      }));
      setRawStats(prev => ({
        top: data.top || prev.top,
        free: data.free || prev.free,
        df: data.df || prev.df,
        net: data.net !== undefined ? data.net : prev.net,
        cores: data.cores !== undefined ? data.cores : prev.cores,
        temp: data.temp !== undefined ? data.temp : prev.temp,
        gpu: data.gpu !== undefined ? data.gpu : prev.gpu,
      }));
    });
    return () => remove();
  }, []);

  return (
    <div className="p-8 h-full overflow-y-auto relative">
      {activeModal === 'cpu' && <CpuModal raw={rawStats.top} cores={rawStats.cores} temp={rawStats.temp} onClose={() => setActiveModal(null)} />}
      {activeModal === 'mem' && <MemModal raw={rawStats.free} onClose={() => setActiveModal(null)} />}
      {activeModal === 'disk' && <DiskModal raw={rawStats.df} onClose={() => setActiveModal(null)} />}
      {activeModal === 'net' && <NetModal raw={rawStats.net} onClose={() => setActiveModal(null)} />}
      {activeModal === 'gpu' && <GpuModal raw={rawStats.gpu} onClose={() => setActiveModal(null)} />}

      <header className="mb-8 flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold text-gray-100">Server Dashboard</h2>
          <p className="text-gray-400 mt-2">Live metrics — click any card for details</p>
        </div>
        {server && (
          <div className="flex items-center gap-4 bg-dark-800/50 p-4 rounded-2xl border border-dark-700">
            <div className="w-12 h-12 bg-dark-900 rounded-full flex items-center justify-center p-2 border border-brand-500/20 shadow-lg shadow-brand-500/10">
              <OsLogo server={server} className="w-full h-full" />
            </div>
            <div>
              <div className="text-lg font-bold text-gray-100">{server.name}</div>
              <div className="text-sm font-mono text-gray-400">
                {server.username}@{server.host}:{server.port}
              </div>
            </div>
          </div>
        )}
      </header>

      {/* Shared Hosting / High Core Warning Banner */}
      {rawStats.cores && rawStats.cores.split('\n').filter(Boolean).length > 16 && (
        <div className="mb-6 flex items-start gap-3 p-4 bg-brand-500/10 border border-brand-500/30 rounded-xl text-brand-300 shadow-sm shadow-brand-500/5">
          <Info size={20} className="shrink-0 mt-0.5" />
          <div className="text-sm">
            <p className="font-semibold mb-1 text-brand-400">High-capacity host detected (Shared Hosting / Large VM)</p>
            <p>Your own container stats may not be available. Displayed metrics (like CPU cores and disk) reflect the entire physical server hardware, not just your specific allocation.</p>
          </div>
        </div>
      )}

      <div className={`grid grid-cols-2 md:grid-cols-3 ${stats.gpu !== null ? 'lg:grid-cols-5' : 'lg:grid-cols-4'} gap-4 mb-8`}>
        <CircularProgress onClick={() => setActiveModal('cpu')} percentage={stats.cpu} label="CPU Usage" />
        <CircularProgress onClick={() => setActiveModal('mem')} percentage={stats.mem} label="Memory" />
        <CircularProgress onClick={() => setActiveModal('disk')} percentage={stats.disk} label="Disk Usage" />
        {stats.gpu !== null && (
          <CircularProgress onClick={() => setActiveModal('gpu')} percentage={stats.gpu} label="GPU Usage" />
        )}
        <NetworkCard raw={rawStats.net} onClick={() => setActiveModal('net')} />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="glass-panel p-6">
          <span className="text-gray-400 text-sm uppercase tracking-wide">System Uptime</span>
          <span className="text-2xl font-semibold mt-2 block text-brand-400">{stats.uptime}</span>
        </div>
        <div className="glass-panel p-6">
          <span className="text-gray-400 text-sm uppercase tracking-wide">Active Users</span>
          <span className="text-2xl font-semibold mt-2 block text-brand-400">{stats.users}</span>
        </div>
      </div>
    </div>
  );
}
