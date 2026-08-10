import express from 'express';
import fs from 'fs';
import os from 'os';
import path from 'path';
import crypto from 'crypto';
import { BrowserWindow } from 'electron';

const generateToken = () => crypto.randomBytes(32).toString('hex');

export function initLocalAPI(vault, sshManagers, windowRoutes, createServerWindow, secretsVault) {
  const app = express();
  app.use(express.json());

  const token = generateToken();
  const glyphDir = path.join(os.homedir(), '.glyph');
  if (!fs.existsSync(glyphDir)) {
    fs.mkdirSync(glyphDir, { recursive: true });
  }
  
  const tokenFile = path.join(glyphDir, 'mcp_token');
  try {
    fs.writeFileSync(tokenFile, token, { mode: 0o600 });
  } catch (err) {
    console.error('Failed to write MCP token:', err);
  }

  // Auth Middleware
  app.use((req, res, next) => {
    const provided = req.headers['authorization'];
    if (provided !== `Bearer ${token}`) {
      return res.status(401).json({ error: 'Unauthorized' });
    }
    next();
  });

  const getConnectedManager = (serverId) => {
    for (const [id, manager] of sshManagers.entries()) {
      const route = windowRoutes.get(id);
      if (route && route.type === 'server' && route.serverId === serverId) {
        if (manager && manager.isConnected) {
          return { manager, winId: id };
        }
      }
    }
    return null;
  };

  const broadcastAgentAction = (serverId, actionTab) => {
    const connected = getConnectedManager(serverId);
    if (!connected) return;
    const win = BrowserWindow.fromId(connected.winId);
    if (win) {
      win.webContents.send('agent-action', actionTab);
    }
  };

  app.get('/api/servers', (req, res) => {
    const servers = vault.getServers().map(s => ({
      id: s.id,
      name: s.name,
      host: s.host,
      username: s.username
    }));
    res.json(servers);
  });

  app.post('/api/connect', async (req, res) => {
    const { serverId } = req.body;
    if (!serverId) return res.status(400).json({ error: 'Missing serverId' });
    
    const config = vault.getServerConfigForConnection(serverId);
    if (!config) return res.status(404).json({ error: 'Server not found in Vault' });

    let connected = getConnectedManager(serverId);
    if (connected) return res.json({ success: true, message: 'Already connected' });

    try {
      createServerWindow(serverId);
      
      const timeout = 20000;
      const start = Date.now();
      while (Date.now() - start < timeout) {
        connected = getConnectedManager(serverId);
        if (connected) return res.json({ success: true });
        await new Promise(r => setTimeout(r, 500));
      }
      res.status(408).json({ error: 'Timed out waiting for server connection' });
    } catch (err) {
      res.status(500).json({ error: err.message });
    }
  });

  app.post('/api/execute', async (req, res) => {
    const { serverId, command } = req.body;
    if (!serverId || !command) return res.status(400).json({ error: 'Missing serverId or command' });
    
    const connected = getConnectedManager(serverId);
    if (!connected) return res.status(400).json({ error: 'Server not connected or window not open' });

    try {
      broadcastAgentAction(serverId, 'terminal');
      const win = BrowserWindow.fromId(connected.winId);
      if (win) {
        win.webContents.send('agent-execute-command', command);
      }
      const output = await connected.manager.exec(command);
      if (win) {
        win.webContents.send('agent-execute-command-output', output);
      }
      res.json({ output });
    } catch (err) {
      res.status(500).json({ error: err.message });
    }
  });

  app.post('/api/sftp/read', async (req, res) => {
    const { serverId, path } = req.body;
    const connected = getConnectedManager(serverId);
    if (!connected) return res.status(400).json({ error: 'Not connected' });

    try {
      const content = await connected.manager.sftpReadFile(path);
      broadcastAgentAction(serverId, 'sftp');
      res.json({ content });
    } catch (err) {
      res.status(500).json({ error: err.message });
    }
  });

  app.post('/api/sftp/write', async (req, res) => {
    const { serverId, path, content } = req.body;
    const connected = getConnectedManager(serverId);
    if (!connected) return res.status(400).json({ error: 'Not connected' });

    try {
      await connected.manager.sftpWriteFile(path, content);
      broadcastAgentAction(serverId, 'sftp');
      res.json({ success: true });
    } catch (err) {
      res.status(500).json({ error: err.message });
    }
  });

  app.post('/api/tunnels/start', async (req, res) => {
    const { serverId, localPort, remoteHost, remotePort, protocol } = req.body;
    if (!serverId || !localPort || !remoteHost || !remotePort) return res.status(400).json({ error: 'Missing parameters' });
    
    const connected = getConnectedManager(serverId);
    if (!connected) return res.status(400).json({ error: 'Server not connected' });

    try {
      const result = await connected.manager.startLocalTunnel(
        parseInt(localPort, 10),
        remoteHost,
        parseInt(remotePort, 10),
        protocol || 'tcp'
      );
      broadcastAgentAction(serverId, 'tunnels');
      res.json(result);
    } catch (err) {
      res.status(500).json({ error: err.message });
    }
  });

  app.post('/api/tunnels/stop', async (req, res) => {
    const { serverId, localPort } = req.body;
    const connected = getConnectedManager(serverId);
    if (!connected) return res.status(400).json({ error: 'Server not connected' });

    try {
      await connected.manager.stopLocalTunnel(parseInt(localPort, 10));
      broadcastAgentAction(serverId, 'tunnels');
      res.json({ success: true });
    } catch (err) {
      res.status(500).json({ error: err.message });
    }
  });

  app.post('/api/secrets/list', (req, res) => {
    const { serverId } = req.body;
    if (!serverId) return res.status(400).json({ error: 'Missing serverId' });
    try {
      broadcastAgentAction(serverId, 'secrets');
      const secrets = secretsVault.getSecrets(serverId);
      res.json(secrets);
    } catch (err) {
      res.status(500).json({ error: err.message });
    }
  });

  app.post('/api/secrets/add', (req, res) => {
    const { serverId, name, value } = req.body;
    if (!serverId || !name || !value) return res.status(400).json({ error: 'Missing parameters' });
    try {
      broadcastAgentAction(serverId, 'secrets');
      secretsVault.addSecret(serverId, name, value);

      const connected = getConnectedManager(serverId);
      if (connected) {
        const win = BrowserWindow.fromId(connected.winId);
        if (win) {
          win.webContents.executeJavaScript(`window.dispatchEvent(new CustomEvent("secretsUpdated"))`);
        }
      }

      res.json({ success: true });
    } catch (err) {
      res.status(500).json({ error: err.message });
    }
  });

  app.post('/api/containers/list', async (req, res) => {
    const { serverId } = req.body;
    const connected = getConnectedManager(serverId);
    if (!connected) return res.status(400).json({ error: 'Server not connected' });
    try {
      broadcastAgentAction(serverId, 'containers');
      const cmd = `docker ps -a --format '{"id":"{{.ID}}", "image":"{{.Image}}", "name":"{{.Names}}", "status":"{{.Status}}", "state":"{{.State}}"}' 2>&1`;
      const output = await connected.manager.exec(cmd);
      res.json({ output });
    } catch (err) {
      res.status(500).json({ error: err.message });
    }
  });

  app.post('/api/commands/list', async (req, res) => {
    const { serverId } = req.body;
    const connected = getConnectedManager(serverId);
    if (!connected) return res.status(400).json({ error: 'Server not connected' });
    try {
      broadcastAgentAction(serverId, 'commands');
      const win = BrowserWindow.fromId(connected.winId);
      if (win) {
        const storageKey = `glyph_commands_${serverId}`;
        const data = await win.webContents.executeJavaScript(`localStorage.getItem("${storageKey}") || localStorage.getItem("glyph_commands")`);
        res.json({ commands: data ? JSON.parse(data) : [] });
      } else {
        res.json({ commands: [] });
      }
    } catch (err) {
      res.status(500).json({ error: err.message });
    }
  });

  app.post('/api/commands/add', async (req, res) => {
    const { serverId, name, cmd } = req.body;
    const connected = getConnectedManager(serverId);
    if (!connected) return res.status(400).json({ error: 'Server not connected' });
    try {
      broadcastAgentAction(serverId, 'commands');
      const win = BrowserWindow.fromId(connected.winId);
      if (win) {
        const storageKey = `glyph_commands_${serverId}`;
        await win.webContents.executeJavaScript(`
          (function() {
            const saved = localStorage.getItem("${storageKey}") || localStorage.getItem("glyph_commands");
            let cmds = saved ? JSON.parse(saved) : [
              { id: 1, name: 'Update System', cmd: 'sudo apt update && sudo apt upgrade -y', uses: 0 },
              { id: 2, name: 'Check Logs', cmd: 'tail -f /var/log/syslog', uses: 0 },
              { id: 3, name: 'List Ports', cmd: 'netstat -tulpn', uses: 0 }
            ];
            cmds.push({ id: Date.now(), name: "${name}", cmd: "${cmd}", uses: 0 });
            localStorage.setItem("${storageKey}", JSON.stringify(cmds));
            window.dispatchEvent(new CustomEvent("commandsUpdated"));
          })();
        `);
        res.json({ success: true });
      } else {
        res.status(400).json({ error: 'Window not found' });
      }
    } catch (err) {
      res.status(500).json({ error: err.message });
    }
  });

  app.post('/api/commands/remove', async (req, res) => {
    const { serverId, id } = req.body;
    const connected = getConnectedManager(serverId);
    if (!connected) return res.status(400).json({ error: 'Server not connected' });
    try {
      broadcastAgentAction(serverId, 'commands');
      const win = BrowserWindow.fromId(connected.winId);
      if (win) {
        const storageKey = `glyph_commands_${serverId}`;
        await win.webContents.executeJavaScript(`
          (function() {
            const saved = localStorage.getItem("${storageKey}");
            if (saved) {
              let cmds = JSON.parse(saved);
              cmds = cmds.filter(c => c.id !== ${id});
              localStorage.setItem("${storageKey}", JSON.stringify(cmds));
              window.dispatchEvent(new CustomEvent("commandsUpdated"));
            }
          })();
        `);
        res.json({ success: true });
      } else {
        res.status(400).json({ error: 'Window not found' });
      }
    } catch (err) {
      res.status(500).json({ error: err.message });
    }
  });

  app.listen(15354, '127.0.0.1', () => {
    console.log('Glyph Local API (MCP Backend) running on http://127.0.0.1:15354');
  });
}
