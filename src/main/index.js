import { app, shell, BrowserWindow, ipcMain, dialog } from 'electron'
import { join } from 'path'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import SSHManager from './sshManager.js'
import Vault from './vault.js'
import SecretsVault from './secretsVault.js'
import { initUpdater } from './updater.js'
import { encryptData, decryptData } from './cryptoUtil.js'
import fs from 'fs'

let mainManagerWindow = null;
const windows = new Map();
const sshManagers = new Map();
const windowRoutes = new Map();

const vault = new Vault();
const secretsVault = new SecretsVault(vault);

function getSSHManager(event) {
  return sshManagers.get(event.sender.id) || null;
}

function getWindow(event) {
  return windows.get(event.sender.id) || null;
}

function createMainWindow() {
  if (mainManagerWindow) {
    mainManagerWindow.focus();
    return;
  }
  
  mainManagerWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    show: false,
    autoHideMenuBar: true,
    icon: join(__dirname, '../../logo.png'),
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: false,
      contextIsolation: true
    }
  })

  const id = mainManagerWindow.webContents.id;
  windows.set(id, mainManagerWindow);
  windowRoutes.set(id, { type: 'manager' });
  
  mainManagerWindow.on('ready-to-show', () => {
    mainManagerWindow.show();
    initUpdater(mainManagerWindow);
  })

  mainManagerWindow.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url)
    return { action: 'deny' }
  })

  mainManagerWindow.on('closed', () => {
    mainManagerWindow = null;
    windows.delete(id);
    windowRoutes.delete(id);
  });

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainManagerWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainManagerWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

function createServerWindow(serverId) {
  const win = new BrowserWindow({
    width: 1200,
    height: 800,
    show: false,
    autoHideMenuBar: true,
    icon: join(__dirname, '../../logo.png'),
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: false,
      contextIsolation: true
    }
  });

  const id = win.webContents.id;
  windows.set(id, win);
  sshManagers.set(id, new SSHManager());
  windowRoutes.set(id, { type: 'server', serverId });

  win.on('ready-to-show', () => {
    win.show();
  });

  win.on('closed', () => {
    const manager = sshManagers.get(id);
    if (manager) manager.disconnect();
    windows.delete(id);
    sshManagers.delete(id);
    windowRoutes.delete(id);
  });

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    win.loadURL(process.env['ELECTRON_RENDERER_URL']);
  } else {
    win.loadFile(join(__dirname, '../renderer/index.html'));
  }
}

app.whenReady().then(() => {
  electronApp.setAppUserModelId('com.glyph')

  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window)
  })

  createMainWindow()

  app.on('activate', function () {
    if (BrowserWindow.getAllWindows().length === 0) createMainWindow()
  })
})

app.on('window-all-closed', () => {
  for (const manager of sshManagers.values()) {
    manager.disconnect();
  }
  app.quit();
})

ipcMain.handle('get-initial-route', (event) => {
  return windowRoutes.get(event.sender.id) || { type: 'manager' };
});

ipcMain.handle('open-server-window', (event, serverId) => {
  createServerWindow(serverId);
  return true;
});

ipcMain.handle('close-window', (event) => {
  const win = getWindow(event);
  if (win) {
    win.close();
  }
  return true;
});

// IPC Handlers for SSH
ipcMain.handle('ssh-connect', async (event, config) => {
  const manager = getSSHManager(event);
  const win = getWindow(event);
  if (!manager) throw new Error('No SSH Manager for this window');
  return await manager.connect(config, win);
});

ipcMain.handle('ssh-connect-saved', async (event, id) => {
  const manager = getSSHManager(event);
  const win = getWindow(event);
  if (!manager) throw new Error('No SSH Manager for this window');
  const config = vault.getServerConfigForConnection(id);
  if (!config) throw new Error('Server not found');
  const res = await manager.connect(config, win);
  if (res.success && res.os) {
    vault.updateServer(id, { os: res.os });
  }
  return res;
});

ipcMain.handle('get-servers', () => {
  return vault.getServers();
});

ipcMain.handle('add-server', (event, config) => {
  return vault.addServer(config);
});

ipcMain.handle('edit-server', (event, id, config) => {
  vault.editServer(id, config);
  return true;
});

ipcMain.handle('export-servers', async (event, masterPassword) => {
  if (!masterPassword) throw new Error('Master password is required');
  const win = getWindow(event) || mainManagerWindow;
  const { canceled, filePath } = await dialog.showSaveDialog(win, {
    title: 'Export Servers',
    defaultPath: 'glyph_servers.glyph',
    filters: [{ name: 'Glyph Export', extensions: ['glyph', 'json'] }]
  });

  if (canceled || !filePath) return false;

  try {
    const rawData = vault.exportServersData();
    const dataStr = JSON.stringify(rawData);
    const encrypted = encryptData(dataStr, masterPassword);
    
    fs.writeFileSync(filePath, encrypted);
    return true;
  } catch (error) {
    console.error('Export error:', error);
    throw new Error('Failed to export servers: ' + error.message);
  }
});

ipcMain.handle('import-servers', async (event, masterPassword) => {
  if (!masterPassword) throw new Error('Master password is required');
  const win = getWindow(event) || mainManagerWindow;
  const { canceled, filePaths } = await dialog.showOpenDialog(win, {
    title: 'Import Servers',
    properties: ['openFile'],
    filters: [{ name: 'Glyph Export', extensions: ['glyph', 'json'] }]
  });

  if (canceled || filePaths.length === 0) return false;

  try {
    const filePath = filePaths[0];
    const encryptedContent = fs.readFileSync(filePath, 'utf8');
    const decryptedStr = decryptData(encryptedContent, masterPassword);
    
    const serversList = JSON.parse(decryptedStr);
    const addedCount = vault.importServersData(serversList);
    
    return addedCount;
  } catch (error) {
    console.error('Import error:', error);
    throw new Error('Failed to import servers. Incorrect password or corrupted file.');
  }
});

ipcMain.handle('delete-server', (event, id) => {
  vault.deleteServer(id);
  return true;
});

ipcMain.handle('ssh-disconnect', async (event) => {
  const manager = getSSHManager(event);
  if (manager) return await manager.disconnect();
  return { success: true };
});

// Multi-tab shell IPC
ipcMain.handle('ssh-open-shell', async (event, tabId) => {
  const manager = getSSHManager(event);
  if (!manager) throw new Error('No SSH Manager for this window');
  return await manager.openShell(tabId);
});

ipcMain.handle('ssh-close-shell', (event, tabId) => {
  const manager = getSSHManager(event);
  if (manager) manager.closeShell(tabId);
  return true;
});

ipcMain.on('ssh-shell-data', (event, tabId, data) => {
  const manager = getSSHManager(event);
  if (manager) manager.writeShell(tabId, data);
});

ipcMain.on('ssh-shell-resize', (event, tabId, cols, rows) => {
  const manager = getSSHManager(event);
  if (manager) manager.resizeShell(tabId, cols, rows);
});

// Secrets Vault IPC Handlers
ipcMain.handle('get-secrets', (event, serverId) => {
  return secretsVault.getSecrets(serverId);
});

ipcMain.handle('add-secret', (event, serverId, name, value) => {
  return secretsVault.addSecret(serverId, name, value);
});

ipcMain.handle('delete-secret', (event, id) => {
  secretsVault.deleteSecret(id);
  return true;
});

// Import / Export Secrets
ipcMain.handle('export-secrets', async (event, serverId, masterPassword) => {
  if (!masterPassword) throw new Error('Master password is required');
  const win = getWindow(event) || mainManagerWindow;
  const { canceled, filePath } = await dialog.showSaveDialog(win, {
    title: 'Export Secrets',
    defaultPath: 'glyph_secrets.glyph',
    filters: [{ name: 'Glyph Export', extensions: ['glyph', 'json'] }]
  });

  if (canceled || !filePath) return false;

  try {
    const rawData = secretsVault.exportSecretsData(serverId);
    const dataStr = JSON.stringify(rawData);
    const encrypted = encryptData(dataStr, masterPassword);
    
    fs.writeFileSync(filePath, encrypted);
    return true;
  } catch (error) {
    console.error('Export error:', error);
    throw new Error('Failed to export secrets: ' + error.message);
  }
});

ipcMain.handle('import-secrets', async (event, serverId, masterPassword) => {
  if (!masterPassword) throw new Error('Master password is required');
  const win = getWindow(event) || mainManagerWindow;
  const { canceled, filePaths } = await dialog.showOpenDialog(win, {
    title: 'Import Secrets',
    properties: ['openFile'],
    filters: [{ name: 'Glyph Export', extensions: ['glyph', 'json'] }]
  });

  if (canceled || filePaths.length === 0) return false;

  try {
    const filePath = filePaths[0];
    const encryptedContent = fs.readFileSync(filePath, 'utf8');
    const decryptedStr = decryptData(encryptedContent, masterPassword);
    
    const secretsList = JSON.parse(decryptedStr);
    const addedCount = secretsVault.importSecretsData(serverId, secretsList);
    
    return addedCount;
  } catch (error) {
    console.error('Import error:', error);
    throw new Error('Failed to import secrets. Incorrect password or corrupted file.');
  }
});

ipcMain.on('inject-secret', (event, id) => {
  const manager = getSSHManager(event);
  const decrypted = secretsVault.getDecryptedSecretValue(id);
  if (decrypted && manager) {
    manager.writeShell(decrypted);
  } else {
    console.error('Failed to inject secret: secret not found or decryption failed.');
  }
});

ipcMain.handle('is-encryption-available', () => {
  const { safeStorage } = require('electron');
  return safeStorage.isEncryptionAvailable();
});

ipcMain.handle('ssh-exec', async (event, command) => {
  const manager = getSSHManager(event);
  if (!manager) return null;
  try {
    return await manager.exec(command);
  } catch (err) {
    if (err.message === 'Not connected') return null;
    throw err;
  }
});

ipcMain.handle('ssh-sftp-readdir', async (event, path) => {
  const manager = getSSHManager(event);
  if (!manager) return null;
  try {
    return await manager.readDir(path);
  } catch (err) {
    if (err.message === 'Not connected') return null;
    throw err;
  }
});

ipcMain.handle('ssh-sftp-read-file', async (event, path) => {
  const manager = getSSHManager(event);
  if (!manager) return null;
  try {
    return await manager.sftpReadFile(path);
  } catch (err) {
    if (err.message === 'Not connected') return null;
    throw err;
  }
});

ipcMain.handle('ssh-sftp-write-file', async (event, path, content) => {
  const manager = getSSHManager(event);
  if (!manager) return null;
  try {
    return await manager.sftpWriteFile(path, content);
  } catch (err) {
    if (err.message === 'Not connected') return null;
    throw err;
  }
});

ipcMain.handle('ssh-sftp-download', async (event, remotePath, filename) => {
  const manager = getSSHManager(event);
  const win = getWindow(event);
  if (!manager) throw new Error('Not connected');
  const { canceled, filePath } = await dialog.showSaveDialog(win, {
    title: 'Download File',
    defaultPath: filename,
  });
  if (canceled || !filePath) return false;
  
  await manager.sftpDownloadFile(remotePath, filePath, (transferred, total) => {
    event.sender.send('sftp-transfer-progress', {
      type: 'download',
      filename,
      transferred,
      total
    });
  });
  return true;
});

ipcMain.handle('ssh-sftp-upload', async (event, remoteDir) => {
  const manager = getSSHManager(event);
  const win = getWindow(event);
  if (!manager) throw new Error('Not connected');
  const { canceled, filePaths } = await dialog.showOpenDialog(win, {
    title: 'Upload File',
    properties: ['openFile']
  });
  if (canceled || filePaths.length === 0) return false;
  
  const localPath = filePaths[0];
  const { basename } = require('path');
  const filename = basename(localPath);
  const remotePath = remoteDir.endsWith('/') ? `${remoteDir}${filename}` : `${remoteDir}/${filename}`;
  
  await manager.sftpUploadFile(localPath, remotePath, (transferred, total) => {
    event.sender.send('sftp-transfer-progress', {
      type: 'upload',
      filename,
      transferred,
      total
    });
  });
  return true;
});

ipcMain.handle('ssh-sftp-upload-dropped', async (event, localPaths, remoteDir) => {
  const manager = getSSHManager(event);
  if (!manager) throw new Error('Not connected');
  const fs = require('fs');
  const path = require('path');

  async function uploadRecursive(localItemPath, remoteItemDir) {
    const stats = fs.statSync(localItemPath);
    const filename = path.basename(localItemPath);
    const targetRemotePath = remoteItemDir.endsWith('/') ? `${remoteItemDir}${filename}` : `${remoteItemDir}/${filename}`;

    if (stats.isDirectory()) {
      // Create remote directory
      await manager.exec(`mkdir -p "${targetRemotePath}"`);
      // Read local directory contents
      const items = fs.readdirSync(localItemPath);
      for (const item of items) {
        await uploadRecursive(path.join(localItemPath, item), targetRemotePath);
      }
    } else {
      // Upload file
      await manager.sftpUploadFile(localItemPath, targetRemotePath, (transferred, total) => {
        event.sender.send('sftp-transfer-progress', {
          type: 'upload',
          filename,
          transferred,
          total
        });
      });
    }
  }

  for (const localPath of localPaths) {
    await uploadRecursive(localPath, remoteDir);
  }
  return true;
});

ipcMain.handle('ssh-start-tunnel', async (event, localPort, remoteHost, remotePort) => {
  const manager = getSSHManager(event);
  if (!manager) throw new Error('Not connected');
  return await manager.startLocalTunnel(localPort, remoteHost, remotePort);
});

ipcMain.handle('ssh-stop-tunnel', async (event, localPort) => {
  const manager = getSSHManager(event);
  if (!manager) throw new Error('Not connected');
  return await manager.stopLocalTunnel(localPort);
});

ipcMain.handle('ssh-get-tunnels', (event) => {
  const manager = getSSHManager(event);
  if (!manager) return [];
  return manager.getActiveTunnels();
});

ipcMain.handle('get-app-version', () => {
  return app.getVersion();
});

ipcMain.handle('get-zt-node-id', async () => {
  try {
    const zt = require('libzt');
    const ztPath = join(app.getPath('userData'), 'zt_node');
    
    try {
      await zt.node.start({ path: ztPath });
    } catch(e) {
      const msg = (e && e.message) ? e.message : String(e);
      if (!msg.includes('already been started')) {
        throw e;
      }
    }
    
    let id = zt.node.id().toString(16);
    let attempts = 0;
    while (id === '0' && attempts < 50) {
      await new Promise(r => setTimeout(r, 100));
      id = zt.node.id().toString(16);
      attempts++;
    }
    
    return id !== '0' ? id : null;
  } catch (e) {
    console.error('Failed to get ZT Node ID:', e);
    return null;
  }
});
