import React, { useEffect, useRef, useState } from 'react';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { SearchAddon } from 'xterm-addon-search';
import { Palette, Zap, Search, Minus, Plus, Trash2, Download, ChevronDown, ArrowUp, ArrowDown, X, Lock } from 'lucide-react';
import 'xterm/css/xterm.css';

const THEMES = {
  'Glyph Default': { background: '#0f111a', foreground: '#f3f4f6', cursor: '#6366f1', selectionBackground: '#818cf850' },
  'Dracula': { background: '#282a36', foreground: '#f8f8f2', cursor: '#ff79c6', selectionBackground: '#44475a' },
  'One Dark': { background: '#282c34', foreground: '#abb2bf', cursor: '#528bff', selectionBackground: '#3e4451' },
  'Solarized Dark': { background: '#002b36', foreground: '#839496', cursor: '#93a1a1', selectionBackground: '#073642' },
  'Monokai': { background: '#272822', foreground: '#f8f8f2', cursor: '#f8f8f0', selectionBackground: '#49483e' }
};

export default function TerminalPage({ server }) {
  const terminalRef = useRef(null);
  const xtermRef = useRef(null);
  const fitAddonRef = useRef(null);
  const searchAddonRef = useRef(null);
  const isDisposedRef = useRef(false);

  const [fontSize, setFontSize] = useState(14);
  const [activeTheme, setActiveTheme] = useState('Glyph Default');
  const [showSearch, setShowSearch] = useState(false);
  const [searchText, setSearchText] = useState('');
  const [quickCommands, setQuickCommands] = useState([]);
  const [showCmds, setShowCmds] = useState(false);
  const [showThemes, setShowThemes] = useState(false);
  const [vaultSecrets, setVaultSecrets] = useState([]);
  const [showSecrets, setShowSecrets] = useState(false);

  // Load quick commands for this server
  useEffect(() => {
    const storageKey = server ? `glyph_commands_${server.id}` : 'glyph_commands';
    const saved = localStorage.getItem(storageKey);
    if (saved) {
      setQuickCommands(JSON.parse(saved).sort((a, b) => (b.uses || 0) - (a.uses || 0)));
    }
  }, [server]);

  // Terminal initialization
  useEffect(() => {
    if (!terminalRef.current || xtermRef.current) return;
    isDisposedRef.current = false;

    const term = new Terminal({
      cursorBlink: true,
      theme: THEMES[activeTheme],
      fontFamily: '"Fira Code", monospace',
      fontSize: fontSize,
    });

    const fitAddon = new FitAddon();
    const searchAddon = new SearchAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(searchAddon);
    term.open(terminalRef.current);

    xtermRef.current = term;
    fitAddonRef.current = fitAddon;
    searchAddonRef.current = searchAddon;

    const safefit = () => {
      if (isDisposedRef.current) return;
      try {
        if (!terminalRef.current || terminalRef.current.offsetParent === null) return;
        if (terminalRef.current.clientWidth > 0 && terminalRef.current.clientHeight > 0) {
          fitAddon.fit();
          window.api.sshShellResize(term.cols, term.rows);
        }
      } catch (e) {}
    };

    const observer = new ResizeObserver(() => setTimeout(safefit, 50));
    observer.observe(terminalRef.current);
    window.addEventListener('resize', safefit);
    setTimeout(safefit, 200);

    term.onData(data => {
      if (!isDisposedRef.current) window.api.sshShellData(data);
    });

    const removeListener = window.api.onSshShellOutput(data => {
      if (!isDisposedRef.current && xtermRef.current) term.write(data);
    });

    term.attachCustomKeyEventHandler((e) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'c' && term.hasSelection()) {
        navigator.clipboard.writeText(term.getSelection());
        return false;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'v') {
        navigator.clipboard.readText().then(text => {
          if (!isDisposedRef.current) window.api.sshShellData(text);
        });
        return false;
      }
      return true;
    });

    return () => {
      isDisposedRef.current = true;
      observer.disconnect();
      window.removeEventListener('resize', safefit);
      removeListener();
      term.dispose();
      xtermRef.current = null;
    };
  }, []);

  // Update theme dynamically
  useEffect(() => {
    if (xtermRef.current) {
      xtermRef.current.options.theme = THEMES[activeTheme];
    }
  }, [activeTheme]);

  // Update font size dynamically
  useEffect(() => {
    if (xtermRef.current && fitAddonRef.current) {
      xtermRef.current.options.fontSize = fontSize;
      setTimeout(() => {
        try { fitAddonRef.current.fit(); window.api.sshShellResize(xtermRef.current.cols, xtermRef.current.rows); } catch(e){}
      }, 50);
    }
  }, [fontSize]);

  const execCommand = (cmdStr) => {
    if (cmdStr && !isDisposedRef.current) {
      window.api.sshShellData(cmdStr + '\n');
    }
    setShowCmds(false);
  };

  const injectSecret = (id) => {
    if (!isDisposedRef.current) {
      window.api.injectSecret(id);
    }
    setShowSecrets(false);
  };

  const loadSecrets = async () => {
    const list = await window.api.getSecrets();
    setVaultSecrets(list);
  };

  return (
    <div className="w-full h-full flex flex-col overflow-hidden relative" style={{ backgroundColor: THEMES[activeTheme].background }}>
      {/* Top Toolbar */}
      <div className="h-12 bg-dark-800/80 backdrop-blur-md border-b border-dark-700 flex items-center justify-between px-4 shrink-0 z-50">
        <div className="flex items-center gap-2">
          
          {/* Quick Commands Dropdown */}
          <div className="relative">
            <button onClick={() => { setShowCmds(!showCmds); setShowThemes(false); }} className="px-3 py-1.5 bg-dark-700 hover:bg-brand-500 hover:text-white text-gray-300 rounded-md text-xs font-semibold flex items-center gap-2 transition-colors">
              <Zap size={14} /> Snippets <ChevronDown size={14} />
            </button>
            {showCmds && (
              <div className="absolute top-full mt-2 left-0 w-64 bg-dark-800 border border-dark-600 rounded-lg shadow-xl overflow-hidden py-1 z-50">
                {quickCommands.length === 0 ? (
                  <div className="px-4 py-3 text-xs text-gray-400">No commands saved for this server. Use the Commands tab!</div>
                ) : (
                  quickCommands.map(c => (
                    <button key={c.id} onClick={() => execCommand(c.cmd)} className="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-brand-500 hover:text-white transition-colors truncate">
                      {c.name}
                    </button>
                  ))
                )}
              </div>
            )}
          </div>

          {/* Secrets Dropdown */}
          <div className="relative ml-2">
            <button onClick={() => { setShowSecrets(!showSecrets); setShowCmds(false); setShowThemes(false); loadSecrets(); }} className="px-3 py-1.5 bg-dark-700 hover:bg-brand-500 hover:text-white text-gray-300 rounded-md text-xs font-semibold flex items-center gap-2 transition-colors">
              <Lock size={14} /> Secrets <ChevronDown size={14} />
            </button>
            {showSecrets && (
              <div className="absolute top-full mt-2 left-0 w-64 bg-dark-800 border border-dark-600 rounded-lg shadow-xl overflow-hidden py-1 z-50">
                {vaultSecrets.length === 0 ? (
                  <div className="px-4 py-3 text-xs text-gray-400">Your vault is empty. Add a secret in the Secrets tab!</div>
                ) : (
                  vaultSecrets.map(s => (
                    <button key={s.id} onClick={() => injectSecret(s.id)} className="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-brand-500 hover:text-white transition-colors flex items-center justify-between">
                      <span className="truncate">{s.name}</span>
                      <span className="text-[10px] uppercase bg-dark-900/50 px-1.5 py-0.5 rounded text-gray-400">Inject</span>
                    </button>
                  ))
                )}
              </div>
            )}
          </div>

          {/* Theme Dropdown */}
          <div className="relative ml-2">
            <button onClick={() => { setShowThemes(!showThemes); setShowCmds(false); }} className="px-3 py-1.5 bg-dark-700 hover:bg-dark-600 text-gray-300 rounded-md text-xs font-semibold flex items-center gap-2 transition-colors">
              <Palette size={14} /> Theme <ChevronDown size={14} />
            </button>
            {showThemes && (
              <div className="absolute top-full mt-2 left-0 w-48 bg-dark-800 border border-dark-600 rounded-lg shadow-xl overflow-hidden py-1 z-50">
                {Object.keys(THEMES).map(t => (
                  <button key={t} onClick={() => { setActiveTheme(t); setShowThemes(false); }} className={`w-full text-left px-4 py-2 text-sm transition-colors ${activeTheme === t ? 'bg-brand-500 text-white' : 'text-gray-300 hover:bg-dark-700'}`}>
                    {t}
                  </button>
                ))}
              </div>
            )}
          </div>

          <div className="w-px h-6 bg-dark-700 mx-2"></div>
          
          <button onClick={() => setFontSize(f => Math.max(8, f - 1))} className="p-1.5 text-gray-400 hover:text-gray-200 hover:bg-dark-700 rounded-md transition-colors" title="Decrease Font Size"><Minus size={16} /></button>
          <span className="text-xs font-mono text-gray-400 w-6 text-center">{fontSize}</span>
          <button onClick={() => setFontSize(f => Math.min(32, f + 1))} className="p-1.5 text-gray-400 hover:text-gray-200 hover:bg-dark-700 rounded-md transition-colors" title="Increase Font Size"><Plus size={16} /></button>
        </div>
        
        <div className="flex items-center gap-2">
          <button onClick={() => {
            if (showSearch) { setShowSearch(false); searchAddonRef.current?.clearDecorations(); }
            else setShowSearch(true);
          }} className={`p-2 rounded-md transition-colors ${showSearch ? 'bg-brand-500 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-dark-700'}`} title="Search (Ctrl+F)">
            <Search size={16} />
          </button>
          
          <div className="w-px h-6 bg-dark-700 mx-1"></div>
          
          <button onClick={() => { if (xtermRef.current) xtermRef.current.clear(); }} className="p-2 text-gray-400 hover:text-red-400 hover:bg-dark-700 rounded-md transition-colors" title="Clear Terminal">
            <Trash2 size={16} />
          </button>
        </div>
      </div>

      {/* Search Bar Overlay */}
      {showSearch && (
        <div className="absolute top-14 right-4 bg-dark-800 border border-dark-600 shadow-xl rounded-lg flex items-center p-1.5 z-20">
          <input 
            autoFocus
            value={searchText}
            onChange={(e) => {
              setSearchText(e.target.value);
              searchAddonRef.current?.findNext(e.target.value);
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                if (e.shiftKey) searchAddonRef.current?.findPrevious(searchText);
                else searchAddonRef.current?.findNext(searchText);
              }
            }}
            className="bg-transparent border-none focus:outline-none text-sm text-gray-200 px-2 w-48"
            placeholder="Find..."
          />
          <button onClick={() => searchAddonRef.current?.findPrevious(searchText)} className="p-1.5 text-gray-400 hover:text-gray-200 hover:bg-dark-700 rounded-md"><ArrowUp size={14} /></button>
          <button onClick={() => searchAddonRef.current?.findNext(searchText)} className="p-1.5 text-gray-400 hover:text-gray-200 hover:bg-dark-700 rounded-md"><ArrowDown size={14} /></button>
          <div className="w-px h-4 bg-dark-700 mx-1"></div>
          <button onClick={() => { setShowSearch(false); searchAddonRef.current?.clearDecorations(); setSearchText(''); }} className="p-1.5 text-gray-400 hover:text-gray-200 hover:bg-dark-700 rounded-md"><X size={14} /></button>
        </div>
      )}

      {/* Click outside overlays to close them */}
      {(showCmds || showThemes || showSecrets) && (
        <div className="absolute inset-0 z-40" onClick={() => { setShowCmds(false); setShowThemes(false); setShowSecrets(false); }}></div>
      )}

      <div className="flex-1 w-full relative z-0 p-2">
        <div ref={terminalRef} className="absolute inset-2 overflow-hidden" />
      </div>
    </div>
  );
}
