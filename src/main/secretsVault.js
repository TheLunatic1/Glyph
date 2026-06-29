import { safeStorage, app } from 'electron';
import fs from 'fs';
import path from 'path';

export default class SecretsVault {
  constructor() {
    this.dataPath = path.join(app.getPath('userData'), 'glyph_secrets.json');
    this.secrets = this.loadSecrets();
  }

  loadSecrets() {
    try {
      if (fs.existsSync(this.dataPath)) {
        const data = fs.readFileSync(this.dataPath, 'utf-8');
        return JSON.parse(data);
      }
    } catch (e) {
      console.error('Error loading secrets', e);
    }
    return [];
  }

  saveSecrets() {
    try {
      fs.writeFileSync(this.dataPath, JSON.stringify(this.secrets, null, 2));
    } catch (e) {
      console.error('Error saving secrets', e);
    }
  }

  getSecrets() {
    // DO NOT return the value. Only ID and Name.
    return this.secrets.map(secret => ({
      id: secret.id,
      name: secret.name
    }));
  }

  addSecret(name, value) {
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

    this.secrets.push(newSecret);
    this.saveSecrets();
    return newSecret.id;
  }

  deleteSecret(id) {
    this.secrets = this.secrets.filter(s => s.id !== id);
    this.saveSecrets();
  }

  getDecryptedSecretValue(id) {
    const secret = this.secrets.find(s => s.id === id);
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
}
