import { contextBridge, ipcRenderer } from 'electron'
import { electronAPI } from '@electron-toolkit/preload'

// Custom APIs for renderer
const api = {
  getInitialRoute: () => ipcRenderer.invoke('get-initial-route'),
  openServerWindow: (id) => ipcRenderer.invoke('open-server-window', id),
  closeWindow: () => ipcRenderer.invoke('close-window'),
  sshConnect: (config) => ipcRenderer.invoke('ssh-connect', config),
  sshConnectSaved: (id) => ipcRenderer.invoke('ssh-connect-saved', id),
  getServers: () => ipcRenderer.invoke('get-servers'),
  addServer: (config) => ipcRenderer.invoke('add-server', config),
  editServer: (id, config) => ipcRenderer.invoke('edit-server', id, config),
  exportServers: (password) => ipcRenderer.invoke('export-servers', password),
  importServers: (password) => ipcRenderer.invoke('import-servers', password),
  deleteServer: (id) => ipcRenderer.invoke('delete-server', id),
  sshDisconnect: () => ipcRenderer.invoke('ssh-disconnect'),
  sshOpenShell: (tabId) => ipcRenderer.invoke('ssh-open-shell', tabId),
  sshCloseShell: (tabId) => ipcRenderer.invoke('ssh-close-shell', tabId),
  sshShellData: (tabId, data) => ipcRenderer.send('ssh-shell-data', tabId, data),
  sshShellResize: (tabId, cols, rows) => ipcRenderer.send('ssh-shell-resize', tabId, cols, rows),
  sshExec: (command) => ipcRenderer.invoke('ssh-exec', command),
  sshSftpReaddir: (path) => ipcRenderer.invoke('ssh-sftp-readdir', path),
  sshSftpReadFile: (path) => ipcRenderer.invoke('ssh-sftp-read-file', path),
  sshSftpWriteFile: (path, content) => ipcRenderer.invoke('ssh-sftp-write-file', path, content),
  sshSftpDownload: (remotePath, filename) => ipcRenderer.invoke('ssh-sftp-download', remotePath, filename),
  sshSftpUpload: (remoteDir) => ipcRenderer.invoke('ssh-sftp-upload', remoteDir),
  sshSftpUploadDropped: (localPaths, remoteDir) => ipcRenderer.invoke('ssh-sftp-upload-dropped', localPaths, remoteDir),
  sshStartTunnel: (localPort, remoteHost, remotePort, protocol) => ipcRenderer.invoke('ssh-start-tunnel', localPort, remoteHost, remotePort, protocol),
  sshStopTunnel: (local) => ipcRenderer.invoke('ssh-stop-tunnel', local),
  sshGetTunnels: () => ipcRenderer.invoke('ssh-get-tunnels'),
  getZtNodeId: () => ipcRenderer.invoke('get-zt-node-id'),
  getAppVersion: () => ipcRenderer.invoke('get-app-version'),
  
  onSshShellOutput: (callback) => {
    const listener = (event, data) => callback(data);
    ipcRenderer.on('ssh-shell-output', listener);
    return () => ipcRenderer.removeListener('ssh-shell-output', listener);
  },

  onSshShellOutputTab: (callback) => {
    const listener = (event, tabId, data) => callback(tabId, data);
    ipcRenderer.on('ssh-shell-output-tab', listener);
    return () => ipcRenderer.removeListener('ssh-shell-output-tab', listener);
  },

  onSshShellClosed: (callback) => {
    const listener = (event, tabId) => callback(tabId);
    ipcRenderer.on('ssh-shell-closed', listener);
    return () => ipcRenderer.removeListener('ssh-shell-closed', listener);
  },
  
  onSshStats: (callback) => {
    const listener = (event, data) => callback(data);
    ipcRenderer.on('ssh-stats', listener);
    return () => ipcRenderer.removeListener('ssh-stats', listener);
  },
  
  onSshStatus: (callback) => {
    const subscription = (event, status) => callback(status);
    ipcRenderer.on('ssh-status', subscription);
    return () => ipcRenderer.removeListener('ssh-status', subscription);
  },

  onSftpProgress: (callback) => {
    const listener = (event, data) => callback(data);
    ipcRenderer.on('sftp-transfer-progress', listener);
    return () => ipcRenderer.removeListener('sftp-transfer-progress', listener);
  },
  
  // Secrets Vault
  getSecrets: (serverId) => ipcRenderer.invoke('get-secrets', serverId),
  addSecret: (serverId, name, value) => ipcRenderer.invoke('add-secret', serverId, name, value),
  exportSecrets: (serverId, password) => ipcRenderer.invoke('export-secrets', serverId, password),
  importSecrets: (serverId, password) => ipcRenderer.invoke('import-secrets', serverId, password),
  deleteSecret: (id) => ipcRenderer.invoke('delete-secret', id),
  injectSecret: (id) => ipcRenderer.send('inject-secret', id),

  // Encryption status
  isEncryptionAvailable: () => ipcRenderer.invoke('is-encryption-available'),

  // Fix #1: Unexpected disconnect notification
  onSshDisconnected: (callback) => {
    const listener = (event, reason) => callback(reason);
    ipcRenderer.on('ssh-disconnected', listener);
    return () => ipcRenderer.removeListener('ssh-disconnected', listener);
  },

  // ── Auto-Updater ────────────────────────────────────────────────────────────
  updaterCheck:    () => ipcRenderer.invoke('updater-check'),
  updaterDownload: () => ipcRenderer.invoke('updater-download'),
  updaterInstall:  () => ipcRenderer.invoke('updater-install'),

  onUpdaterAvailable: (callback) => {
    const listener = (_, info) => callback(info);
    ipcRenderer.on('updater-available', listener);
    return () => ipcRenderer.removeListener('updater-available', listener);
  },
  onUpdaterProgress: (callback) => {
    const listener = (_, progress) => callback(progress);
    ipcRenderer.on('updater-progress', listener);
    return () => ipcRenderer.removeListener('updater-progress', listener);
  },
  onUpdaterDownloaded: (callback) => {
    const listener = () => callback();
    ipcRenderer.on('updater-downloaded', listener);
    return () => ipcRenderer.removeListener('updater-downloaded', listener);
  },
  onUpdaterError: (callback) => {
    const listener = (_, msg) => callback(msg);
    ipcRenderer.on('updater-error', listener);
    return () => ipcRenderer.removeListener('updater-error', listener);
  },
}

if (process.contextIsolated) {
  try {
    contextBridge.exposeInMainWorld('electron', electronAPI)
    contextBridge.exposeInMainWorld('api', api)
  } catch (error) {
    console.error(error)
  }
} else {
  window.electron = electronAPI
  window.api = api
}
