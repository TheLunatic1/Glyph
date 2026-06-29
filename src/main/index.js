import { app, shell, BrowserWindow, ipcMain, dialog } from 'electron'
import { join } from 'path'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import SSHManager from './sshManager.js'
import Vault from './vault.js'
import SecretsVault from './secretsVault.js'

let mainWindow;
const sshManager = new SSHManager();
const vault = new Vault();
const secretsVault = new SecretsVault();

function createWindow() {
  mainWindow = new BrowserWindow({
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

  mainWindow.on('ready-to-show', () => {
    mainWindow.show()
  })

  mainWindow.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url)
    return { action: 'deny' }
  })

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

app.whenReady().then(() => {
  electronApp.setAppUserModelId('com.glyph')

  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window)
  })

  createWindow()

  app.on('activate', function () {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  sshManager.disconnect();
  app.quit();
})

// IPC Handlers for SSH
ipcMain.handle('ssh-connect', async (event, config) => {
  return await sshManager.connect(config, mainWindow);
});

ipcMain.handle('ssh-connect-saved', async (event, id) => {
  const config = vault.getServerConfigForConnection(id);
  if (!config) throw new Error('Server not found');
  const res = await sshManager.connect(config, mainWindow);
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

ipcMain.handle('delete-server', (event, id) => {
  vault.deleteServer(id);
  return true;
});

ipcMain.handle('ssh-disconnect', async () => {
  return await sshManager.disconnect();
});

ipcMain.on('ssh-shell-data', (event, data) => {
  sshManager.writeShell(data);
});

ipcMain.on('ssh-shell-resize', (event, cols, rows) => {
  sshManager.resizeShell(cols, rows);
});

// Secrets Vault IPC Handlers
ipcMain.handle('get-secrets', () => {
  return secretsVault.getSecrets();
});

ipcMain.handle('add-secret', (event, name, value) => {
  return secretsVault.addSecret(name, value);
});

ipcMain.handle('delete-secret', (event, id) => {
  secretsVault.deleteSecret(id);
  return true;
});

ipcMain.on('inject-secret', (event, id) => {
  const decrypted = secretsVault.getDecryptedSecretValue(id);
  if (decrypted) {
    sshManager.writeShell(decrypted);
  } else {
    console.error('Failed to inject secret: secret not found or decryption failed.');
  }
});

ipcMain.handle('ssh-exec', async (event, command) => {
  return await sshManager.exec(command);
});

ipcMain.handle('ssh-sftp-readdir', async (event, path) => {
  return await sshManager.readDir(path);
});

ipcMain.handle('ssh-sftp-read-file', async (event, path) => {
  return await sshManager.sftpReadFile(path);
});

ipcMain.handle('ssh-sftp-write-file', async (event, path, content) => {
  return await sshManager.sftpWriteFile(path, content);
});

ipcMain.handle('ssh-start-tunnel', async (event, localPort, remoteHost, remotePort) => {
  return await sshManager.startLocalTunnel(localPort, remoteHost, remotePort);
});

ipcMain.handle('ssh-stop-tunnel', async (event, localPort) => {
  return await sshManager.stopLocalTunnel(localPort);
});

ipcMain.handle('ssh-get-tunnels', () => {
  return sshManager.getActiveTunnels();
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
