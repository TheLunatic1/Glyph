import { safeStorage, app } from 'electron';
import fs from 'fs';
import path from 'path';

export default class SecretsVault {
  constructor(vault) {
    this.dataPath = path.join(app.getPath('userData'), 'glyph_secrets_server.json');
    this.vault = vault;
    this.secrets = this.loadSecrets();
  }

  loadSecrets() {
    try {
      if (fs.existsSync(this.dataPath)) {
        const data = fs.readFileSync(this.dataPath, 'utf-8');
        return JSON.parse(data);
      }
    } catch (e) {
      console.error('Error loading server secrets', e);
    }
    
    // Migration logic
    const oldPath = path.join(app.getPath('userData'), 'glyph_secrets.json');
    if (fs.existsSync(oldPath)) {
      try {
        const oldSecrets = JSON.parse(fs.readFileSync(oldPath, 'utf-8'));
        const servers = this.vault ? this.vault.getServers() : [];
        const firstServerId = servers.length > 0 ? servers[0].id : 'global';
        const migrated = {};
        if (oldSecrets.length > 0) {
           migrated[firstServerId] = oldSecrets;
        }
        
        fs.writeFileSync(this.dataPath, JSON.stringify(migrated, null, 2));
        fs.renameSync(oldPath, oldPath + '.backup');
        return migrated;
      } catch (e) {
        console.error('Failed to migrate old secrets', e);
      }
    }
    
    return {};
  }

  saveSecrets() {
    try {
      fs.writeFileSync(this.dataPath, JSON.stringify(this.secrets, null, 2));
    } catch (e) {
      console.error('Error saving secrets', e);
    }
  }

  getSecrets(serverId) {
    if (!serverId) return [];
    const serverSecrets = this.secrets[serverId] || [];
    return serverSecrets.map(secret => ({
      id: secret.id,
      name: secret.name
    }));
  }

  addSecret(serverId, name, value) {
    if (!serverId) throw new Error('serverId is required');
    
    const newSecret = {
      id: Date.now().toString(),
      name: name
    };

    if (safeStorage.isEncryptionAvailable()) {
      const encrypted = safeStorage.encryptString(value);
      newSecret.value = encrypted.toString('base64');
    } else {
      console.warn('[SecretsVault] safeStorage unavailable — secret stored as base64 (NOT encrypted).');
      newSecret.valueFallback = Buffer.from(value).toString('base64');
    }

    if (!this.secrets[serverId]) {
      this.secrets[serverId] = [];
    }
    this.secrets[serverId].push(newSecret);
    this.saveSecrets();
    return newSecret.id;
  }

  deleteSecret(id) {
    for (const serverId of Object.keys(this.secrets)) {
      const initialLen = this.secrets[serverId].length;
      this.secrets[serverId] = this.secrets[serverId].filter(s => s.id !== id);
      if (this.secrets[serverId].length < initialLen) {
         this.saveSecrets();
         return;
      }
    }
  }

  getDecryptedSecretValue(id) {
    let secret = null;
    for (const serverId of Object.keys(this.secrets)) {
      secret = this.secrets[serverId].find(s => s.id === id);
      if (secret) break;
    }
    
    if (!secret) return null;

    if (secret.value && safeStorage.isEncryptionAvailable()) {
      try {
        const buffer = Buffer.from(secret.value, 'base64');
        return safeStorage.decryptString(buffer);
      } catch (e) {
        console.error('Failed to decrypt secret', e);
        return null;
      }
    } else if (secret.valueFallback) {
      return Buffer.from(secret.valueFallback, 'base64').toString('utf-8');
    }
    return null;
  }

  exportSecretsData(serverId) {
    return this.secrets[serverId] || [];
  }

  importSecretsData(serverId, secretsList) {
    if (!serverId || !Array.isArray(secretsList)) return 0;
    
    if (!this.secrets[serverId]) {
      this.secrets[serverId] = [];
    }
    
    let count = 0;
    for (const secret of secretsList) {
       if (secret.name && (secret.value || secret.valueFallback)) {
          const newSecret = { ...secret, id: Date.now().toString() + Math.random().toString(36).substr(2, 5) };
          this.secrets[serverId].push(newSecret);
          count++;
       }
    }
    this.saveSecrets();
    return count;
  }
}
