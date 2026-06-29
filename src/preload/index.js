import { contextBridge, ipcRenderer } from 'electron'
import { electronAPI } from '@electron-toolkit/preload'

// Custom APIs for renderer
const api = {
  sshConnect: (config) => ipcRenderer.invoke('ssh-connect', config),
  sshConnectSaved: (id) => ipcRenderer.invoke('ssh-connect-saved', id),
  getServers: () => ipcRenderer.invoke('get-servers'),
  addServer: (config) => ipcRenderer.invoke('add-server', config),
  deleteServer: (id) => ipcRenderer.invoke('delete-server', id),
  sshDisconnect: () => ipcRenderer.invoke('ssh-disconnect'),
  sshShellData: (data) => ipcRenderer.send('ssh-shell-data', data),
  sshShellResize: (cols, rows) => ipcRenderer.send('ssh-shell-resize', cols, rows),
  sshExec: (command) => ipcRenderer.invoke('ssh-exec', command),
  sshSftpReaddir: (path) => ipcRenderer.invoke('ssh-sftp-readdir', path),
  getZtNodeId: () => ipcRenderer.invoke('get-zt-node-id'),
  getAppVersion: () => ipcRenderer.invoke('get-app-version'),
  
  onSshShellOutput: (callback) => {
    const listener = (event, data) => callback(data);
    ipcRenderer.on('ssh-shell-output', listener);
    return () => ipcRenderer.removeListener('ssh-shell-output', listener);
  },
  
  onSshStats: (callback) => {
    const listener = (event, data) => callback(data);
    ipcRenderer.on('ssh-stats', listener);
    return () => ipcRenderer.removeListener('ssh-stats', listener);
  },
  
  onSshStatus: (callback) => {
    const listener = (event, data) => callback(data);
    ipcRenderer.on('ssh-status', listener);
    return () => ipcRenderer.removeListener('ssh-status', listener);
  }
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
