import React, { useState, useEffect } from 'react';
import {
  X, Smartphone, Download, QrCode, ExternalLink, Copy, Check,
  Zap, ShieldCheck, Terminal, FolderOpen, Box, Sparkles, RefreshCw
} from 'lucide-react';
import QRCode from 'qrcode';

const DEFAULT_REPO = 'https://github.com/TheLunatic1/glyph-app';
const RELEASES_API = 'https://api.github.com/repos/TheLunatic1/glyph-app/releases/latest';
const DEFAULT_DOWNLOAD_URL = 'https://github.com/TheLunatic1/glyph-app/releases/latest';

function fmtBytes(bytes) {
  if (!bytes) return null;
  if (bytes >= 1e9) return (bytes / 1e9).toFixed(2) + ' GB';
  if (bytes >= 1e6) return (bytes / 1e6).toFixed(1) + ' MB';
  if (bytes >= 1e3) return (bytes / 1e3).toFixed(1) + ' KB';
  return bytes + ' B';
}

export default function MobileAppModal({ visible, onClose }) {
  const [releaseInfo, setReleaseInfo] = useState({
    version: 'v1.0.1',
    apkName: 'Glyph-Mobile-Android-v1.0.1.apk',
    downloadUrl: DEFAULT_DOWNLOAD_URL,
    releaseUrl: DEFAULT_REPO,
    size: null,
    publishedAt: null
  });
  const [qrCodeDataUrl, setQrCodeDataUrl] = useState('');
  const [copied, setCopied] = useState(false);
  const [loading, setLoading] = useState(true);
  const [qrMode, setQrMode] = useState('direct'); // 'direct' | 'release'

  useEffect(() => {
    if (!visible) return;
    loadLatestRelease();
  }, [visible]);

  const loadLatestRelease = async () => {
    setLoading(true);
    try {
      const res = await fetch(RELEASES_API);
      if (res.ok) {
        const data = await res.json();
        const tag = data.tag_name || 'v1.0.1';
        let apkAsset = null;
        if (Array.isArray(data.assets)) {
          apkAsset = data.assets.find(a => a.name?.endsWith('.apk'));
        }

        const directUrl = apkAsset?.browser_download_url || data.html_url || DEFAULT_DOWNLOAD_URL;
        const info = {
          version: tag.startsWith('v') ? tag : `v${tag}`,
          apkName: apkAsset?.name || `Glyph-Mobile-${tag}.apk`,
          downloadUrl: directUrl,
          releaseUrl: data.html_url || DEFAULT_REPO,
          size: apkAsset?.size || null,
          publishedAt: data.published_at
        };
        setReleaseInfo(info);
        generateQr(qrMode === 'direct' ? info.downloadUrl : info.releaseUrl);
      } else {
        generateQr(DEFAULT_DOWNLOAD_URL);
      }
    } catch (err) {
      console.warn('[MobileAppModal] Failed to fetch latest release:', err);
      generateQr(DEFAULT_DOWNLOAD_URL);
    } finally {
      setLoading(false);
    }
  };

  const generateQr = async (url) => {
    try {
      const dataUrl = await QRCode.toDataURL(url, {
        width: 240,
        margin: 1.5,
        color: {
          dark: '#0f172a',
          light: '#ffffff'
        },
        errorCorrectionLevel: 'M'
      });
      setQrCodeDataUrl(dataUrl);
    } catch (err) {
      console.error('[MobileAppModal] QR generation failed:', err);
    }
  };

  const handleToggleMode = (mode) => {
    setQrMode(mode);
    const targetUrl = mode === 'direct' ? releaseInfo.downloadUrl : releaseInfo.releaseUrl;
    generateQr(targetUrl);
  };

  const handleCopyLink = () => {
    const targetUrl = qrMode === 'direct' ? releaseInfo.downloadUrl : releaseInfo.releaseUrl;
    navigator.clipboard.writeText(targetUrl);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  if (!visible) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-md p-4 animate-in fade-in duration-200"
      onClick={onClose}
    >
      <div
        className="bg-dark-900 border border-brand-500/30 w-full max-w-2xl rounded-2xl shadow-2xl overflow-hidden flex flex-col relative"
        onClick={e => e.stopPropagation()}
      >
        {/* Glow Accent Bar */}
        <div className="h-1 w-full bg-gradient-to-r from-brand-500 via-purple-500 to-cyan-400" />

        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-dark-800 bg-dark-800/40">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-brand-500/20 to-purple-500/20 border border-brand-500/40 flex items-center justify-center shadow-inner">
              <Smartphone size={22} className="text-brand-400" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-gray-100 font-bold text-lg leading-tight">Glyph Mobile</h2>
                <span className="px-2 py-0.5 rounded-full text-[10px] font-bold bg-brand-500/20 text-brand-300 border border-brand-500/30">
                  Android APK
                </span>
                <span className="px-2 py-0.5 rounded-full text-[10px] font-mono font-semibold bg-dark-700 text-gray-300 border border-dark-600">
                  {releaseInfo.version}
                </span>
              </div>
              <p className="text-gray-400 text-xs mt-0.5">
                Take your secure SSH servers, telemetry & Docker controls anywhere
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 text-gray-500 hover:text-white hover:bg-dark-700 rounded-lg transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {/* Body */}
        <div className="p-6 flex flex-col md:flex-row gap-6 max-h-[80vh] overflow-y-auto custom-scrollbar">
          {/* Left Column: QR Code Box */}
          <div className="flex flex-col items-center md:w-60 shrink-0">
            <div className="p-3 bg-white rounded-2xl shadow-xl shadow-brand-500/10 border-2 border-brand-500/30 relative group">
              {qrCodeDataUrl ? (
                <img
                  src={qrCodeDataUrl}
                  alt="Scan to Download Glyph Mobile APK"
                  className="w-48 h-48 rounded-xl object-contain"
                />
              ) : (
                <div className="w-48 h-48 flex items-center justify-center bg-gray-100 rounded-xl">
                  <RefreshCw size={24} className="animate-spin text-gray-400" />
                </div>
              )}
            </div>

            {/* Scan Prompt */}
            <div className="text-center mt-3">
              <div className="flex items-center justify-center gap-1.5 text-brand-300 font-semibold text-xs">
                <QrCode size={13} />
                <span>Scan with Camera</span>
              </div>
              <p className="text-[11px] text-gray-500 mt-0.5">
                Auto-downloads latest APK directly to your phone
              </p>
            </div>

            {/* Mode Switcher */}
            <div className="flex items-center gap-1 mt-3 p-1 bg-dark-800 rounded-lg border border-dark-700 text-[11px] w-full">
              <button
                onClick={() => handleToggleMode('direct')}
                className={`flex-1 py-1 px-2 rounded-md font-medium transition-all ${
                  qrMode === 'direct'
                    ? 'bg-brand-500 text-white shadow-sm'
                    : 'text-gray-400 hover:text-gray-200'
                }`}
              >
                Direct APK
              </button>
              <button
                onClick={() => handleToggleMode('release')}
                className={`flex-1 py-1 px-2 rounded-md font-medium transition-all ${
                  qrMode === 'release'
                    ? 'bg-brand-500 text-white shadow-sm'
                    : 'text-gray-400 hover:text-gray-200'
                }`}
              >
                GitHub Page
              </button>
            </div>
          </div>

          {/* Right Column: Features, Metadata & Actions */}
          <div className="flex-1 flex flex-col justify-between">
            <div>
              <p className="text-xs uppercase tracking-wider text-gray-400 font-bold mb-3 flex items-center gap-1.5">
                <Sparkles size={13} className="text-brand-400" />
                100% Desktop Parity on Mobile
              </p>

              {/* Feature Grid */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5 mb-5">
                <div className="flex items-start gap-2.5 p-2.5 bg-dark-800/60 rounded-xl border border-dark-700/60">
                  <Zap size={16} className="text-brand-400 shrink-0 mt-0.5" />
                  <div>
                    <p className="text-xs font-semibold text-gray-200">5-Metric Telemetry</p>
                    <p className="text-[10px] text-gray-400">Live CPU, RAM, Disk, GPU & Net gauges</p>
                  </div>
                </div>

                <div className="flex items-start gap-2.5 p-2.5 bg-dark-800/60 rounded-xl border border-dark-700/60">
                  <Terminal size={16} className="text-cyan-400 shrink-0 mt-0.5" />
                  <div>
                    <p className="text-xs font-semibold text-gray-200">PTY Terminal</p>
                    <p className="text-[10px] text-gray-400">Full ANSI shell with mobile helper keys</p>
                  </div>
                </div>

                <div className="flex items-start gap-2.5 p-2.5 bg-dark-800/60 rounded-xl border border-dark-700/60">
                  <FolderOpen size={16} className="text-amber-400 shrink-0 mt-0.5" />
                  <div>
                    <p className="text-xs font-semibold text-gray-200">SFTP & Editor</p>
                    <p className="text-[10px] text-gray-400">Browse files & edit code on the go</p>
                  </div>
                </div>

                <div className="flex items-start gap-2.5 p-2.5 bg-dark-800/60 rounded-xl border border-dark-700/60">
                  <Box size={16} className="text-emerald-400 shrink-0 mt-0.5" />
                  <div>
                    <p className="text-xs font-semibold text-gray-200">Docker Manager</p>
                    <p className="text-[10px] text-gray-400">Search, restart & stream live logs</p>
                  </div>
                </div>
              </div>

              {/* Vault Sync Highlight */}
              <div className="p-3 bg-brand-500/10 border border-brand-500/25 rounded-xl mb-4 flex items-start gap-3">
                <ShieldCheck size={18} className="text-brand-400 shrink-0 mt-0.5" />
                <div className="text-xs text-gray-300 leading-relaxed">
                  <span className="font-semibold text-brand-300">Seamless Vault Sync:</span> Export your master-password encrypted <span className="font-mono text-white bg-dark-900 px-1 py-0.5 rounded text-[11px]">.glyph</span> backup file from Desktop, transfer it to your phone, and import all your servers in seconds.
                </div>
              </div>
            </div>

            {/* Action Buttons */}
            <div className="space-y-2 pt-2 border-t border-dark-800">
              <div className="flex items-center gap-2">
                <a
                  href={releaseInfo.downloadUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 bg-brand-500 hover:bg-brand-400 text-white text-xs font-bold rounded-xl transition-all shadow-lg shadow-brand-500/20"
                >
                  <Download size={15} />
                  <span>Download APK {releaseInfo.size ? `(${fmtBytes(releaseInfo.size)})` : ''}</span>
                </a>

                <button
                  onClick={handleCopyLink}
                  className="flex items-center gap-1.5 px-3 py-2.5 bg-dark-800 hover:bg-dark-700 border border-dark-700 text-gray-300 text-xs font-semibold rounded-xl transition-all"
                  title="Copy APK download URL"
                >
                  {copied ? <Check size={14} className="text-emerald-400" /> : <Copy size={14} />}
                  <span>{copied ? 'Copied!' : 'Copy Link'}</span>
                </button>

                <a
                  href={releaseInfo.releaseUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="p-2.5 bg-dark-800 hover:bg-dark-700 border border-dark-700 text-gray-400 hover:text-gray-200 rounded-xl transition-all"
                  title="View on GitHub Releases"
                >
                  <ExternalLink size={15} />
                </a>
              </div>

              <p className="text-[10px] text-gray-500 text-center">
                Compatible with Android 10.0+ (ARM64 & x86_64) • Open Source under Apache-2.0
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
