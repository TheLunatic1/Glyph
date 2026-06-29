import { safeStorage, app } from 'electron';
import fs from 'fs';
import path from 'path';

export default class Vault {
  constructor() {
    this.dataPath = path.join(app.getPath('userData'), 'glyph_servers.json');
    this.servers = this.loadServers();
  }

  loadServers() {
    try {
      if (fs.existsSync(this.dataPath)) {
        const data = fs.readFileSync(this.dataPath, 'utf-8');
        return JSON.parse(data);
      }
    } catch (e) {
      console.error('Error loading servers', e);
    }
    return [];
  }

  saveServers() {
    try {
      fs.writeFileSync(this.dataPath, JSON.stringify(this.servers, null, 2));
    } catch (e) {
      console.error('Error saving servers', e);
    }
  }

  getServers() {
    return this.servers.map(server => ({
      id: server.id,
      name: server.name,
      host: server.host,
      username: server.username,
      port: server.port || 22,
      os: server.os || null,
      zerotier: server.zerotier || ''
      // Do not send passwords to the frontend
    }));
  }

  addServer(serverConfig) {
    const newServer = {
      id: Date.now().toString(),
      name: serverConfig.name || serverConfig.host,
      host: serverConfig.host,
      username: serverConfig.username,
      port: serverConfig.port || 22,
      zerotier: serverConfig.zerotier || '',
      privateKeyPath: serverConfig.privateKey || '',
    };

    if (serverConfig.password) {
      if (safeStorage.isEncryptionAvailable()) {
        const encrypted = safeStorage.encryptString(serverConfig.password);
        newServer.password = encrypted.toString('base64');
      } else {
        // WARNING: safeStorage is unavailable on this system. The password is
        // stored as base64 — this is NOT encryption. Anyone with filesystem
        // access to the userData directory can read it. Consider using a
        // system keychain or running Electron with a keychain service.
        console.warn(
          '[Vault] safeStorage unavailable — password stored as base64 (NOT encrypted). ' +
          'See: https://www.electronjs.org/docs/latest/api/safe-storage'
        );
        newServer.passwordFallback = Buffer.from(serverConfig.password).toString('base64');
      }
    }

    this.servers.push(newServer);
    this.saveServers();
    return newServer.id;
  }

  deleteServer(id) {
    this.servers = this.servers.filter(s => s.id !== id);
    this.saveServers();
  }

  updateServer(id, updates) {
    const idx = this.servers.findIndex(s => s.id === id);
    if (idx !== -1) {
      this.servers[idx] = { ...this.servers[idx], ...updates };
      this.saveServers();
    }
  }

  getServerConfigForConnection(id) {
    const server = this.servers.find(s => s.id === id);
    if (!server) return null;

    const config = {
      host: server.host,
      username: server.username,
      port: server.port,
      zerotier: server.zerotier || '',
      privateKeyPath: server.privateKeyPath || '',
    };

    if (server.password && safeStorage.isEncryptionAvailable()) {
      try {
        config.password = safeStorage.decryptString(Buffer.from(server.password, 'base64'));
      } catch(e) {
        console.error('Failed to decrypt password', e);
      }
    } else if (server.passwordFallback) {
      config.password = Buffer.from(server.passwordFallback, 'base64').toString('utf-8');
    }

    return config;
  }
}
