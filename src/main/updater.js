import { autoUpdater } from 'electron-updater';
import { ipcMain } from 'electron';
import { is } from '@electron-toolkit/utils';

let mainWindow = null;

function send(channel, data) {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send(channel, data);
  }
}

export function initUpdater(win) {
  mainWindow = win;

  // In dev mode, the updater cannot work (no installed app, no latest.yml).
  // We bail out silently — the renderer still uses the GitHub API fallback.
  if (is.dev) {
    console.log('[Updater] Dev mode — auto-updater disabled.');
    return;
  }

  // Silent logging in production
  autoUpdater.logger = null;
  autoUpdater.autoDownload = false;      // We download only when the user asks
  autoUpdater.autoInstallOnAppQuit = false;

  // ── Events ────────────────────────────────────────────────────────────────
  autoUpdater.on('checking-for-update', () => {
    send('updater-checking', null);
  });

  autoUpdater.on('update-available', (info) => {
    send('updater-available', {
      version:      info.version,
      releaseNotes: info.releaseNotes || '',
      releaseDate:  info.releaseDate,
    });
  });

  autoUpdater.on('update-not-available', () => {
    send('updater-not-available', null);
  });

  autoUpdater.on('download-progress', (progress) => {
    send('updater-progress', {
      percent:          Math.round(progress.percent),
      bytesPerSecond:   progress.bytesPerSecond,
      transferred:      progress.transferred,
      total:            progress.total,
    });
  });

  autoUpdater.on('update-downloaded', () => {
    send('updater-downloaded', null);
  });

  autoUpdater.on('error', (err) => {
    console.error('[Updater] Error:', err);
    send('updater-error', err?.message || String(err));
  });

  // Kick off silent check shortly after launch
  setTimeout(() => {
    autoUpdater.checkForUpdates().catch((err) => {
      console.error('[Updater] checkForUpdates failed:', err);
    });
  }, 6000);
}

// ── IPC action handlers (called from renderer via preload) ─────────────────
ipcMain.handle('updater-check', async () => {
  if (is.dev) return { dev: true };
  try {
    await autoUpdater.checkForUpdates();
  } catch (err) {
    console.error('[Updater] manual check failed:', err);
  }
});

ipcMain.handle('updater-download', async () => {
  if (is.dev) return;
  try {
    await autoUpdater.downloadUpdate();
  } catch (err) {
    console.error('[Updater] download failed:', err);
    send('updater-error', err?.message || String(err));
  }
});

ipcMain.handle('updater-install', () => {
  if (is.dev) return;
  autoUpdater.quitAndInstall(false, true);
});
