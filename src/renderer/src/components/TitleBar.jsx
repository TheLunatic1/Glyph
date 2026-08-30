import React from 'react';
import logoSrc from '../assets/logo.png';

export default function TitleBar({ title = 'Glyph', showAttribution }) {
  const isServerScreen = title !== 'Glyph';
  const shouldShowAttribution = showAttribution !== undefined ? showAttribution : !isServerScreen;

  return (
    <div
      className="h-8 w-full bg-dark-900 flex items-center shrink-0 border-b border-dark-800 relative z-50 select-none"
      style={{ WebkitAppRegion: 'drag' }}
    >
      <div className="flex items-center gap-2 px-3 h-full">
        <img src={logoSrc} alt="Glyph" className="w-4 h-4 object-contain opacity-80 pointer-events-none" />
        <span className="text-xs font-semibold text-brand-300 tracking-widest pointer-events-none">{title}</span>
        {shouldShowAttribution && (
          <>
            <span className="text-gray-600 text-[10px] pointer-events-none">•</span>
            <div className="flex items-center text-[10px] text-gray-500 font-medium opacity-70">
              Made by
              <a
                href="https://github.com/TheLunatic1"
                target="_blank"
                rel="noopener noreferrer"
                className="text-brand-400 hover:text-brand-300 transition-colors ml-1"
                style={{ WebkitAppRegion: 'no-drag' }}
              >
                TheLunatic1 (Salman Toha)
              </a>
            </div>
          </>
        )}
      </div>

      <div className="flex-1 h-full" />
      {/* Right side is reserved for native OS window controls (via titleBarOverlay) */}
      <div className="w-[140px] shrink-0 h-full" />
    </div>
  );
}
