import React from 'react';
import logoSrc from '../assets/logo.png';

export default function SplashScreen({ state = 'visible' }) {
  if (state === 'hidden') return null;

  return (
    <div
      className={`fixed inset-0 z-[100] bg-dark-900 flex flex-col items-center justify-center select-none transition-opacity duration-500 pointer-events-auto ${
        state === 'fading' ? 'opacity-0 pointer-events-none' : 'opacity-100'
      }`}
      style={{ WebkitAppRegion: 'drag' }}
    >
      <div className="flex flex-col items-center">
        <div className="relative mb-6">
          <div className="absolute -inset-4 bg-brand-500/10 rounded-full blur-xl animate-pulse"></div>
          <img
            src={logoSrc}
            alt="Glyph"
            className="w-20 h-20 rounded-2xl object-contain animate-breathe relative z-10"
          />
        </div>
        
        <h1 className="text-4xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-gray-300 via-white to-gray-300 tracking-[0.25em] animate-wave-text mb-2">
          Glyph
        </h1>
        
        <p className="text-xs text-gray-500 tracking-wider font-medium mb-6">
          Secure SSH & Server Management
        </p>

        <div className="flex items-center gap-1.5">
          <div className="w-1.5 h-1.5 rounded-full bg-brand-400 animate-pulse"></div>
          <div className="w-1.5 h-1.5 rounded-full bg-brand-400 animate-pulse [animation-delay:200ms]"></div>
          <div className="w-1.5 h-1.5 rounded-full bg-brand-400 animate-pulse [animation-delay:400ms]"></div>
        </div>
      </div>
    </div>
  );
}
