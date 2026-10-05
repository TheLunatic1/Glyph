import React, { useState, useEffect } from 'react';
import { Activity, Terminal, FolderOpen, Code, Box, LogOut, Lock, Network, Smartphone, QrCode } from 'lucide-react';
import logoSrc from '../assets/logo.png';

export default function Sidebar({ activeTab, onTabChange, onDisconnect, onOpenMobile }) {
  const [version, setVersion] = useState('');

  useEffect(() => {
    window.api?.getAppVersion().then(v => {
      if (v) setVersion(v);
    });
  }, []);

  const links = [
    { id: 'dashboard', icon: <Activity size={20} />, label: 'Dashboard' },
    { id: 'terminal', icon: <Terminal size={20} />, label: 'Terminal' },
    { id: 'sftp', icon: <FolderOpen size={20} />, label: 'SFTP' },
    { id: 'commands', icon: <Code size={20} />, label: 'Commands' },
    { id: 'secrets', icon: <Lock size={20} />, label: 'Secrets' },
    { id: 'tunnels', icon: <Network size={20} />, label: 'Tunnels' },
    { id: 'containers', icon: <Box size={20} />, label: 'Containers' },
  ];

  return (
    <div className="w-64 h-full bg-dark-800 border-r border-dark-700 flex flex-col pt-6 pb-4">
      <div className="px-6 mb-8 flex items-center gap-3">
        <img src={logoSrc} alt="Glyph" className="w-9 h-9 rounded-lg object-contain animate-breathe" />
        <h1 className="text-xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-gray-400 via-white to-gray-400 tracking-wider animate-wave-text">
          Glyph
        </h1>
      </div>
      
      <nav className="flex-1 px-4 space-y-1 overflow-y-auto">
        {links.map((link) => (
          <button
            key={link.id}
            onClick={() => onTabChange(link.id)}
            className={`flex items-center gap-3 px-4 py-3 rounded-lg transition-all duration-200 w-full text-left tracking-wide ${
              activeTab === link.id
                ? 'bg-brand-500/10 text-brand-400 font-medium'
                : 'text-gray-400 hover:bg-dark-700 hover:text-gray-200'
            }`}
          >
            {link.icon}
            {link.label}
          </button>
        ))}
      </nav>

      <div className="px-4 mt-auto pb-4">
        {onOpenMobile && (
          <button
            onClick={onOpenMobile}
            className="w-full flex items-center gap-2.5 px-3.5 py-2.5 bg-gradient-to-r from-brand-500/10 to-purple-500/10 hover:from-brand-500/20 hover:to-purple-500/20 border border-brand-500/25 rounded-xl text-xs font-semibold text-brand-300 hover:text-white transition-all tracking-wide mb-3 shadow-sm shadow-brand-500/5"
            title="Get Glyph Mobile for Android (QR Code & Direct APK)"
          >
            <Smartphone size={16} className="text-brand-400" />
            <span>Glyph Mobile (APK)</span>
            <QrCode size={13} className="text-gray-500 ml-auto" />
          </button>
        )}
        <button
          onClick={onDisconnect}
          className="w-full flex items-center gap-3 px-4 py-3 text-red-400 hover:bg-red-500/10 rounded-lg transition-colors tracking-wide mb-4"
        >
          <LogOut size={20} />
          Disconnect
        </button>
        <div className="text-center text-xs text-gray-500 font-medium">
          Made by <a href="https://github.com/TheLunatic1" target="_blank" rel="noopener noreferrer" className="text-brand-400 hover:text-brand-300 transition-colors">TheLunatic1 (Salman Toha)</a>
          {version && <span className="block mt-1 text-[11px] text-gray-600 font-mono">v{version}</span>}
        </div>
      </div>
    </div>
  );
}
