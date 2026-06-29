import { Client } from 'ssh2';
import * as net from 'net';

export default class SSHManager {
  constructor() {
    this.client = null;
    this.shellStream = null;
    this.sftp = null;
    this.isConnected = false;
    this.mainWindow = null;
    this.statInterval = null;
    this.activeTunnels = new Map();
  }

  // ── Internal: reset all per-session state ─────────────────────────────────
  _resetState() {
    if (this.statInterval) {
      clearInterval(this.statInterval);
      this.statInterval = null;
    }
    if (this.shellStream) {
      try { this.shellStream.end(); } catch (_) {}
      this.shellStream = null;
    }
    this._closeAllTunnels();
    if (this.sftp) {
      try { this.sftp.end(); } catch (_) {}
      this.sftp = null;
    }
    if (this.client) {
      // Catch any late-firing timeout/socket errors to prevent Uncaught Exceptions
      this.client.on('error', () => {});
      try { 
        this.client.destroy(); 
      } catch (_) {}
      this.client = null;
    }
    this.isConnected = false;
  }

  // ── Connect ────────────────────────────────────────────────────────────────
  connect(config, mainWindow) {
    this.mainWindow = mainWindow;

    // Always start fresh — fixes re-use-after-end and listener accumulation
    this._resetState();
    this.client = new Client();

    return new Promise((resolve, reject) => {
      this.client
        .on('ready', async () => {
          this.isConnected = true;
          this.initShell();
          this.initSFTP();
          this.startStatPolling();

          let os = null;
          try {
            // Multi-fallback OS detection: handles OpenWrt, Alpine, BSD, standard Linux
            const osRaw = await this.exec(
              'if [ -f /etc/openwrt_release ]; then' +
              '  echo openwrt; ' +
              'elif [ -f /etc/os-release ]; then' +
              '  grep "^ID=" /etc/os-release 2>/dev/null | head -1 | cut -d= -f2 | tr -d \'"\'; ' +
              'elif [ -f /etc/alpine-release ]; then' +
              '  echo alpine; ' +
              'elif [ -f /etc/gentoo-release ]; then' +
              '  echo gentoo; ' +
              'elif [ -f /etc/freebsd-version ] || uname -s 2>/dev/null | grep -qi freebsd; then' +
              '  echo freebsd; ' +
              'else' +
              '  uname -s 2>/dev/null | tr "[:upper:]" "[:lower:]"; ' +
              'fi 2>/dev/null'
            );
            if (osRaw) os = osRaw.trim().toLowerCase().split('\n')[0]; // take first line only
          } catch (e) {
            console.error('OS detection failed:', e);
          }

          resolve({ success: true, os });
        })
        .on('error', (err) => {
          resolve({ success: false, error: err.message });
        })
        .on('end', () => {
          this.isConnected = false;
        })
        .on('close', () => {
          this.isConnected = false;
        });

      // Kick off connection (ZeroTier path needs async setup)
      this._buildConnectOpts(config)
        .then((connectOpts) => {
          this.client.connect(connectOpts);
        })
        .catch((err) => {
          resolve({ success: false, error: err.message || String(err) });
        });
    });
  }

  // ── Build connect options (handles ZeroTier) ──────────────────────────────
  async _buildConnectOpts(config) {
    if (this.mainWindow) {
      this.mainWindow.webContents.send('ssh-status', `Initializing connection to ${config.host}...`);
    }

    const connectOpts = {
      host: config.host,
      port: config.port || 22,
      username: config.username,
      readyTimeout: 10000,
    };

    if (config.password) connectOpts.password = config.password;

    // Load private key from disk if a path was provided
    if (config.privateKeyPath) {
      try {
        const fs = require('fs');
        connectOpts.privateKey = fs.readFileSync(config.privateKeyPath);
      } catch (e) {
        throw new Error(`Could not read private key at "${config.privateKeyPath}": ${e.message}`);
      }
    } else if (config.privateKey) {
      connectOpts.privateKey = config.privateKey;
    }

    if (config.zerotier) {
      const zt = require('libzt');
      const { app } = require('electron');
      const path = require('path');

      const ztPath = path.join(app.getPath('userData'), 'zt_node');

      console.log(`Starting ZeroTier node and joining network ${config.zerotier}...`);
      if (this.mainWindow) {
        this.mainWindow.webContents.send('ssh-status', `Starting ZeroTier node...`);
        this.mainWindow.webContents.send('ssh-status', `Joining Network ID: ${config.zerotier}`);
      }

      try {
        await zt.node.start({ path: ztPath });
      } catch (e) {
        const msg = (e && e.message) ? e.message : String(e);
        if (!msg.includes('already been started')) throw e;
      }

      let ip = null;
      try {
        ip = await zt.node.getIPv4Address(config.zerotier);
      } catch (_) {}

      if (!ip) {
        if (this.mainWindow) {
          this.mainWindow.webContents.send('ssh-status', `Waiting for network authorization...`);
        }
        await zt.node.joinNetwork(config.zerotier);
        for (let i = 0; i < 30; i++) {
          try {
            ip = await zt.node.getIPv4Address(config.zerotier);
            break;
          } catch (_) {
            await new Promise((r) => setTimeout(r, 1000));
          }
        }
        if (!ip) throw new Error('Timed out waiting for ZeroTier IP (did you authorize the node?)');
      }

      console.log(`Creating ZeroTier socket to ${config.host}:${config.port}`);
      if (this.mainWindow) {
        this.mainWindow.webContents.send('ssh-status', `ZeroTier IP acquired: ${ip}`);
        this.mainWindow.webContents.send('ssh-status', `Creating TCP tunnel over ZT to ${config.host}:${config.port}`);
      }
      const sock = zt.net.createConnection(connectOpts.port, connectOpts.host);
      sock.on('error', (err) => {
        console.error('ZeroTier socket error:', err);
      });
      connectOpts.sock = sock;
      delete connectOpts.host;
      delete connectOpts.port;
    }

    return connectOpts;
  }

  // ── Disconnect ─────────────────────────────────────────────────────────────
  async disconnect() {
    this._resetState();
    return { success: true };
  }

  // ── Shell ──────────────────────────────────────────────────────────────────
  initShell() {
    if (!this.client || !this.isConnected) return;
    this.client.shell({ term: 'xterm-256color' }, (err, stream) => {
      if (err) { console.error('Shell error:', err); return; }
      this.shellStream = stream;
      stream
        .on('close', () => { this.shellStream = null; })
        .on('data', (data) => {
          if (this.mainWindow) {
            this.mainWindow.webContents.send('ssh-shell-output', data.toString('utf8'));
          }
        });
    });
  }

  // ── SFTP ───────────────────────────────────────────────────────────────────
  initSFTP() {
    if (!this.client || !this.isConnected) return;
    this.client.sftp((err, sftp) => {
      if (!err) this.sftp = sftp;
      else console.error('SFTP init error:', err);
    });
  }

  writeShell(data) {
    if (this.shellStream) this.shellStream.write(data);
  }

  resizeShell(cols, rows) {
    if (this.shellStream) this.shellStream.setWindow(rows, cols, 0, 0);
  }

  // ── Exec ───────────────────────────────────────────────────────────────────
  async exec(command) {
    this.execLock = this.execLock || Promise.resolve();
    
    // Chain the new command onto the end of the lock queue
    const nextTask = this.execLock.then(() => {
      return new Promise((resolve, reject) => {
        if (!this.isConnected || !this.client) return reject(new Error('Not connected'));
        this.client.exec(command, (err, stream) => {
          if (err) return reject(err);
          let output = '';
          stream
            .on('close', () => resolve(output))
            .on('data', (data) => { output += data; })
            .stderr.on('data', (data) => { output += data; });
        });
      });
    }).catch(err => {
      // Prevent a failed command from breaking the entire queue
      throw err;
    });

    // Update the lock to point to this task, but catch errors so the next task can still run
    this.execLock = nextTask.catch(() => {});
    
    return nextTask;
  }

  // ── SFTP readdir ───────────────────────────────────────────────────────────
  async readDir(path) {
    return new Promise((resolve, reject) => {
      if (!this.sftp) return reject(new Error('SFTP not initialized'));
      this.sftp.readdir(path, (err, list) => {
        if (err) { reject(err); return; }
        const serializedList = list.map((item) => ({
          filename: item.filename,
          isDirectory: item.attrs.isDirectory(),
          size: item.attrs.size,
          modifyTime: item.attrs.mtime,
        }));
        resolve(serializedList);
      });
    });
  }

  // ── SFTP Read File ────────────────────────────────────────────────────────
  async sftpReadFile(path) {
    return new Promise((resolve, reject) => {
      if (!this.sftp) return reject(new Error('SFTP not initialized'));
      this.sftp.readFile(path, 'utf8', (err, data) => {
        if (err) return reject(err);
        resolve(data);
      });
    });
  }

  // ── SFTP Write File ───────────────────────────────────────────────────────
  async sftpWriteFile(path, content) {
    return new Promise((resolve, reject) => {
      if (!this.sftp) return reject(new Error('SFTP not initialized'));
      this.sftp.writeFile(path, content, 'utf8', (err) => {
        if (err) return reject(err);
        resolve(true);
      });
    });
  }

  // ── Stat polling ───────────────────────────────────────────────────────────
  startStatPolling() {
    if (this.statInterval) clearInterval(this.statInterval);
    const INTERVAL_MS = 3000;

    let prevNetBytes = {};
    let prevCpuStats = {};
    let lastPollTime = Date.now();

    this.statInterval = setInterval(async () => {
      if (!this.isConnected) return;
      try {
        const now     = Date.now();
        const elapsed = Math.max(1, (now - lastPollTime) / 1000);
        lastPollTime  = now;

        // Batch all commands into a single SSH exec to avoid MaxSessions limit
        const bashCmd = `
          top -b -n 1 | head -n 8
          echo "===GLYPH_DELIMITER==="
          free -m
          echo "===GLYPH_DELIMITER==="
          df -h --output=source,size,used,avail,pcent,target 2>/dev/null || df -h
          echo "===GLYPH_DELIMITER==="
          uptime -p
          echo "===GLYPH_DELIMITER==="
          who | awk '{print $1}' | sort -u | wc -l
          echo "===GLYPH_DELIMITER==="
          awk 'NR>2{print $1,$2,$10}' /proc/net/dev 2>/dev/null
          echo "===GLYPH_DELIMITER==="
          grep '^cpu' /proc/stat 2>/dev/null
          echo "===GLYPH_DELIMITER==="
          sensors 2>/dev/null
          echo "===GLYPH_DELIMITER==="
          paste <(ls /sys/class/thermal/thermal_zone*/type 2>/dev/null) <(cat /sys/class/thermal/thermal_zone*/temp 2>/dev/null) 2>/dev/null | awk '{type=$1; val=$2; printf "%s %dC\\n", type, val/1000}' | head -8
          echo "===GLYPH_DELIMITER==="
          if command -v nvidia-smi >/dev/null 2>&1; then
            nvidia-smi --query-gpu=index,name,utilization.gpu,temperature.gpu,memory.total,memory.used --format=csv,noheader 2>/dev/null | sed 's/ MiB//g; s/ %//g'
          else
            idx=0
            found=0
            for card in /sys/class/drm/card[0-9]*; do
              if [ -d "$card" ]; then
                vendor=$(cat "$card/device/vendor" 2>/dev/null)
                if [ "$vendor" = "0x1002" ]; then
                  util=$(cat "$card/device/gpu_busy_percent" 2>/dev/null || echo "0")
                  temp_input=$(cat "$card/device/hwmon/hwmon"*/temp1_input 2>/dev/null | head -n1 || echo "0")
                  temp=$((temp_input / 1000))
                  mem_total=$(cat "$card/device/mem_info_vram_total" 2>/dev/null || echo "0")
                  mem_used=$(cat "$card/device/mem_info_vram_used" 2>/dev/null || echo "0")
                  echo "$idx, AMD GPU, $util, $temp, $((mem_total/1024/1024)), $((mem_used/1024/1024))"
                  idx=$((idx+1))
                  found=1
                elif [ "$vendor" = "0x8086" ]; then
                  echo "$idx, Intel GPU, 0, 0, 0, 0"
                  idx=$((idx+1))
                  found=1
                fi
              fi
            done
            if [ $found -eq 0 ]; then echo "NO_GPU"; fi
          fi
        `;
        const rawOutput = await this.exec(bashCmd);
        const parts = rawOutput.split('===GLYPH_DELIMITER===').map((s) => s.trim());

        const topOutput    = parts[0] || '';
        const freeOutput   = parts[1] || '';
        const dfOutput     = parts[2] || '';
        const uptimeOutput = parts[3] || '';
        const usersOutput  = parts[4] || '';
        const rawNetLines  = parts[5] || '';
        const statRaw      = parts[6] || '';
        const sensorsRaw   = parts[7] || '';
        const tzTemps      = parts[8] || '';
        const gpuRaw       = parts[9] || '';

        // ── Network speed (delta bytes / elapsed seconds) ──────────────────
        const currentBytes = {};
        const speedLines   = [];
        if (rawNetLines) {
          rawNetLines.split('\n').forEach((line) => {
            const p = line.trim().split(/\s+/);
            if (p.length >= 3) {
              const iface  = p[0].replace(':', '');
              const rx     = parseInt(p[1], 10) || 0;
              const tx     = parseInt(p[2], 10) || 0;
              currentBytes[iface] = { rx, tx };
              const rxSpeed = prevNetBytes[iface] ? Math.max(0, (rx - prevNetBytes[iface].rx) / elapsed) : 0;
              const txSpeed = prevNetBytes[iface] ? Math.max(0, (tx - prevNetBytes[iface].tx) / elapsed) : 0;
              speedLines.push(`${iface} RX:${rx} TX:${tx} RXS:${rxSpeed.toFixed(0)} TXS:${txSpeed.toFixed(0)}`);
            }
          });
        }
        prevNetBytes = currentBytes;

        // ── Per-core CPU usage + aggregate total (delta from /proc/stat) ──────
        const coreLines  = [];
        const currentCpu = {};
        let   totalCpuPct = -1; // -1 = not computed yet (first tick)
        if (statRaw) {
          statRaw.split('\n').forEach((line) => {
            const p    = line.trim().split(/\s+/);
            const name = p[0];
            // ^cpu$ = aggregate total; ^cpu\d+$ = per-core
            if (!/^cpu\d*$/.test(name)) return;

            const user = parseInt(p[1], 10) || 0, nice = parseInt(p[2], 10) || 0,
                  sys  = parseInt(p[3], 10) || 0, idle = parseInt(p[4], 10) || 0,
                  iow  = parseInt(p[5], 10) || 0, irq  = parseInt(p[6], 10) || 0,
                  sirq = parseInt(p[7], 10) || 0;
            currentCpu[name] = { user, nice, sys, idle, iow, irq, sirq };

            if (prevCpuStats[name]) {
              const prev   = prevCpuStats[name];
              const dIdle  = (idle + iow) - (prev.idle + prev.iow);
              const dTotal = (user + nice + sys + idle + iow + irq + sirq)
                           - (prev.user + prev.nice + prev.sys + prev.idle + prev.iow + prev.irq + prev.sirq);
              const pct    = dTotal > 0 ? Math.max(0, Math.min(100, ((dTotal - dIdle) / dTotal) * 100)) : 0;

              if (name === 'cpu') {
                totalCpuPct = pct; // Accurate aggregate from /proc/stat
              } else {
                coreLines.push(`${name} ${pct.toFixed(1)}`);
              }
            } else {
              if (name !== 'cpu') coreLines.push(`${name} 0.0`);
            }
          });
          prevCpuStats = currentCpu;
        }

        // ── Temperature ────────────────────────────────────────────────────
        let tempOutput = '';
        if (sensorsRaw) {
          const filtered = sensorsRaw.split('\n').filter((l) => /[+\-]?\d+\.\d+.{0,2}C/.test(l));
          tempOutput = filtered.join('\n');
        }
        if (!tempOutput.trim() && tzTemps) tempOutput = tzTemps;

        if (this.mainWindow) {
          this.mainWindow.webContents.send('ssh-stats', {
            top:    topOutput,
            free:   freeOutput,
            df:     dfOutput,
            uptime: uptimeOutput,
            users:  usersOutput,
            net:    speedLines.join('\n'),
            cores:  coreLines.join('\n'),
            temp:   tempOutput,
            // cpuPct: accurate aggregate from /proc/stat (-1 on first tick = no data yet)
            cpuPct: totalCpuPct,
            gpu:    gpuRaw,
          });
        }
      } catch (_) {
        // silent fail — polling must never crash
      }
    }, INTERVAL_MS);
  }

  // ── Port Forwarding (Tunnels) ──────────────────────────────────────────────
  async startLocalTunnel(localPort, remoteHost, remotePort) {
    return new Promise((resolve, reject) => {
      if (!this.isConnected || !this.client) {
        return reject(new Error('SSH client is not connected'));
      }
      if (this.activeTunnels.has(localPort)) {
        return reject(new Error(`Local port ${localPort} is already in use by an active tunnel.`));
      }

      const connections = new Set();

      const server = net.createServer((socket) => {
        connections.add(socket);
        socket.on('close', () => connections.delete(socket));
        this.client.forwardOut(
          '127.0.0.1', localPort,
          remoteHost, remotePort,
          (err, stream) => {
            if (err) {
              console.error('forwardOut error:', err);
              socket.end();
              return;
            }
            socket.pipe(stream);
            stream.pipe(socket);
          }
        );
      });

      server.listen(localPort, '127.0.0.1', () => {
        this.activeTunnels.set(localPort, {
          server,
          connections,
          remoteHost,
          remotePort
        });
        resolve({ success: true, localPort, remoteHost, remotePort });
      });

      server.on('error', (err) => {
        reject(err);
      });
    });
  }

  async stopLocalTunnel(localPort) {
    return new Promise((resolve, reject) => {
      const tunnel = this.activeTunnels.get(localPort);
      if (!tunnel) return resolve(true);

      // Forcefully destroy any lingering connections (e.g. browser HTTP keep-alives)
      for (const socket of tunnel.connections) {
        socket.destroy();
      }

      tunnel.server.close((err) => {
        if (err) {
          console.error(`Error closing tunnel on port ${localPort}:`, err);
          return reject(err);
        }
        this.activeTunnels.delete(localPort);
        resolve(true);
      });
    });
  }

  getActiveTunnels() {
    return Array.from(this.activeTunnels.entries()).map(([localPort, data]) => ({
      localPort,
      remoteHost: data.remoteHost,
      remotePort: data.remotePort
    }));
  }

  _closeAllTunnels() {
    for (const [localPort, tunnel] of this.activeTunnels.entries()) {
      try { tunnel.server.close(); } catch (_) {}
    }
    this.activeTunnels.clear();
  }
}
