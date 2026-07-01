import React from 'react';
import { X, Download, RotateCcw, CheckCircle, AlertTriangle, ExternalLink, Zap } from 'lucide-react';
import { marked } from 'marked';

const RELEASES_URL = 'https://github.com/TheLunatic1/Glyph/releases/latest';

function fmtSpeed(bps) {
  if (bps >= 1e6) return (bps / 1e6).toFixed(1) + ' MB/s';
  if (bps >= 1e3) return (bps / 1e3).toFixed(1) + ' KB/s';
  return bps.toFixed(0) + ' B/s';
}

function fmtBytes(bytes) {
  if (bytes >= 1e9) return (bytes / 1e9).toFixed(2) + ' GB';
  if (bytes >= 1e6) return (bytes / 1e6).toFixed(1) + ' MB';
  if (bytes >= 1e3) return (bytes / 1e3).toFixed(1) + ' KB';
  return bytes + ' B';
}

// Render GitHub release notes (plain text / basic markdown / HTML) cleanly
function ReleaseNotes({ notes }) {
  if (!notes) return <p className="text-gray-500 text-sm italic">No release notes available.</p>;
  const text = typeof notes === 'string' ? notes : notes.map?.(n => n.note).join('\n') || '';
  
  // Convert markdown/HTML safely into styled DOM content
  const htmlContent = marked.parse(text);

  return (
    <div
      className="release-notes-prose text-gray-300 text-sm leading-relaxed max-h-56 overflow-y-auto custom-scrollbar pr-2"
      dangerouslySetInnerHTML={{ __html: htmlContent }}
    />
  );
}

export default function UpdateModal({ info, stage, progress, onDownload, onInstall, onRetry, onClose }) {
  // stage: 'available' | 'downloading' | 'downloaded' | 'error'

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-200" onClick={onClose}>
      <div
        className="bg-dark-900 border border-dark-700 w-full max-w-xl rounded-2xl shadow-2xl overflow-hidden flex flex-col"
        onClick={e => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-dark-800 bg-dark-800/40">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-brand-500/10 border border-brand-500/20 flex items-center justify-center">
              <Zap size={20} className="text-brand-400" />
            </div>
            <div>
              <h2 className="text-gray-100 font-bold text-lg leading-tight">
                {stage === 'downloaded'
                  ? 'Ready to Install'
                  : info?.version
                  ? `Glyph v${info.version}`
                  : 'Glyph Update'}
              </h2>
              <p className="text-gray-500 text-xs">
                {stage === 'available'    && 'New version available'}
                {stage === 'downloading'  && 'Downloading update...'}
                {stage === 'downloaded'   && 'Download complete'}
                {stage === 'error'        && 'Update failed'}
              </p>
            </div>
          </div>
          <button onClick={onClose} className="p-2 text-gray-500 hover:text-white hover:bg-dark-700 rounded-lg transition-colors">
            <X size={18} />
          </button>
        </div>

        {/* Body */}
        <div className="p-6 flex flex-col gap-5 overflow-auto">

          {/* ── AVAILABLE ─────────────────────────────────────────────────── */}
          {stage === 'available' && (
            <>
              <div>
                <p className="text-xs uppercase tracking-wider text-gray-500 font-medium mb-2">What's New</p>
                <div className="bg-dark-800 rounded-xl p-4 border border-dark-700">
                  <ReleaseNotes notes={info?.releaseNotes} />
                </div>
              </div>
              {info?.releaseDate && (
                <p className="text-xs text-gray-600">
                  Released: {new Date(info.releaseDate).toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' })}
                </p>
              )}
              <div className="flex gap-3 pt-1">
                <button
                  onClick={onDownload}
                  className="flex-1 flex items-center justify-center gap-2 px-5 py-2.5 bg-brand-500 hover:bg-brand-400 text-white font-semibold rounded-xl transition-colors shadow-lg shadow-brand-500/20"
                >
                  <Download size={18} /> Download & Install
                </button>
                <button
                  onClick={() => window.open(RELEASES_URL, '_blank')}
                  className="flex items-center gap-2 px-4 py-2.5 bg-dark-700 hover:bg-dark-600 text-gray-300 font-medium rounded-xl transition-colors"
                  title="Download manually from GitHub"
                >
                  <ExternalLink size={16} /> Manual
                </button>
              </div>
            </>
          )}

          {/* ── DOWNLOADING ───────────────────────────────────────────────── */}
          {stage === 'downloading' && (
            <div className="flex flex-col gap-4">
              <div>
                <div className="flex justify-between text-xs text-gray-400 mb-2">
                  <span>Downloading v{info?.version}...</span>
                  <span className="font-mono">
                    {progress?.percent ?? 0}% &nbsp;·&nbsp; {progress?.bytesPerSecond ? fmtSpeed(progress.bytesPerSecond) : '—'}
                  </span>
                </div>
                <div className="w-full h-3 bg-dark-700 rounded-full overflow-hidden">
                  <div
                    className="h-3 rounded-full bg-gradient-to-r from-brand-500 to-brand-400 transition-all duration-300"
                    style={{ width: `${progress?.percent ?? 0}%` }}
                  />
                </div>
                {progress?.total > 0 && (
                  <p className="text-xs text-gray-600 mt-1.5">
                    {fmtBytes(progress.transferred ?? 0)} / {fmtBytes(progress.total)}
                  </p>
                )}
              </div>
              <p className="text-gray-500 text-sm">Please keep Glyph open while the update downloads.</p>
            </div>
          )}

          {/* ── DOWNLOADED ────────────────────────────────────────────────── */}
          {stage === 'downloaded' && (
            <div className="flex flex-col items-center gap-5 py-4">
              <div className="w-16 h-16 rounded-2xl bg-green-500/10 border border-green-500/20 flex items-center justify-center">
                <CheckCircle size={36} className="text-green-400" />
              </div>
              <div className="text-center">
                <p className="text-gray-100 font-semibold text-lg">v{info?.version} is ready!</p>
                <p className="text-gray-400 text-sm mt-1">Glyph will close and relaunch with the new version.</p>
              </div>
              <button
                onClick={onInstall}
                className="w-full flex items-center justify-center gap-2 px-5 py-3 bg-green-500 hover:bg-green-400 text-white font-bold rounded-xl transition-colors shadow-lg shadow-green-500/20"
              >
                <RotateCcw size={18} /> Restart & Install
              </button>
            </div>
          )}

          {/* ── ERROR ─────────────────────────────────────────────────────── */}
          {stage === 'error' && (
            <div className="flex flex-col items-center gap-5 py-4">
              <div className="w-16 h-16 rounded-2xl bg-red-500/10 border border-red-500/20 flex items-center justify-center">
                <AlertTriangle size={36} className="text-red-400" />
              </div>
              <div className="text-center">
                <p className="text-gray-100 font-semibold text-lg">Update Failed</p>
                <p className="text-gray-400 text-sm mt-1">Something went wrong while downloading the update.</p>
              </div>
              <div className="flex gap-3 w-full">
                <button
                  onClick={onRetry}
                  className="flex-1 flex items-center justify-center gap-2 px-5 py-2.5 bg-brand-500 hover:bg-brand-400 text-white font-semibold rounded-xl transition-colors"
                >
                  <RotateCcw size={16} /> Try Again
                </button>
                <button
                  onClick={() => window.open(RELEASES_URL, '_blank')}
                  className="flex items-center gap-2 px-4 py-2.5 bg-dark-700 hover:bg-dark-600 text-gray-300 font-medium rounded-xl transition-colors"
                >
                  <ExternalLink size={16} /> Download Manually
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
