import React, { useEffect, useRef, useState, useCallback } from 'react';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { SearchAddon } from 'xterm-addon-search';
import {
  Palette, Zap, Search, Minus, Plus, Trash2, ChevronDown,
  ArrowUp, ArrowDown, X, Lock, SquarePlus
} from 'lucide-react';
import 'xterm/css/xterm.css';

const THEMES = {
  'Glyph Default': { background: '#0f111a', foreground: '#f3f4f6', cursor: '#6366f1', selectionBackground: '#818cf850' },
  'Dracula':       { background: '#282a36', foreground: '#f8f8f2', cursor: '#ff79c6', selectionBackground: '#44475a' },
  'One Dark':      { background: '#282c34', foreground: '#abb2bf', cursor: '#528bff', selectionBackground: '#3e4451' },
  'Solarized Dark':{ background: '#002b36', foreground: '#839496', cursor: '#93a1a1', selectionBackground: '#073642' },
  'Monokai':       { background: '#272822', foreground: '#f8f8f2', cursor: '#f8f8f0', selectionBackground: '#49483e' },
};

let tabCounter = 1; // next tab number for user-created tabs
const FIRST_TAB  = { id: 'tab-main', title: 'Terminal 1' };
const makeTab = () => {
  tabCounter += 1;
  return { id: `tab-${Date.now()}`, title: `Terminal ${tabCounter}` };
};

// ── Single terminal pane (one xterm instance bound to one SSH shell tab) ─────
function TerminalPane({ tabId, server, theme, fontSize, active }) {
  const containerRef = useRef(null);
  const xtermRef     = useRef(null);
  const fitRef       = useRef(null);
  const searchRef    = useRef(null);
  const disposedRef  = useRef(false);
  const openedRef    = useRef(false);

  // Init xterm once on mount
  useEffect(() => {
    if (!containerRef.current || xtermRef.current) return;
    disposedRef.current = false;

    const term = new Terminal({
      cursorBlink: true,
      theme: THEMES[theme],
      fontFamily: '"Fira Code", monospace',
      fontSize,
    });

    const fit    = new FitAddon();
    const search = new SearchAddon();
    term.loadAddon(fit);
    term.loadAddon(search);
    term.open(containerRef.current);

    xtermRef.current  = term;
    fitRef.current    = fit;
    searchRef.current = search;

    const safefit = () => {
      if (disposedRef.current) return;
      try {
        if (!containerRef.current || containerRef.current.offsetParent === null) return;
        if (containerRef.current.clientWidth > 0 && containerRef.current.clientHeight > 0) {
          fit.fit();
          window.api.sshShellResize(tabId, term.cols, term.rows);
        }
      } catch (_) {}
    };

    const observer = new ResizeObserver(() => setTimeout(safefit, 50));
    observer.observe(containerRef.current);
    window.addEventListener('resize', safefit);
    setTimeout(safefit, 200);

    // Input → SSH
    term.onData(data => {
      if (!disposedRef.current) window.api.sshShellData(tabId, data);
    });

    // Copy on selection Ctrl+C / Paste Ctrl+V
    term.attachCustomKeyEventHandler((e) => {
      if (e.type !== 'keydown') return true;
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'c' && term.hasSelection()) {
        navigator.clipboard.writeText(term.getSelection());
        return false;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'v') return false;
      return true;
    });

    // Open the SSH shell for this tab
    window.api.sshOpenShell(tabId).then(() => {
      openedRef.current = true;
      setTimeout(safefit, 100);
    }).catch(err => {
      term.write(`\r\n\x1b[31mFailed to open shell: ${err.message}\x1b[0m\r\n`);
    });

    return () => {
      disposedRef.current = true;
      observer.disconnect();
      window.removeEventListener('resize', safefit);
      term.dispose();
      xtermRef.current = null;
      if (openedRef.current) window.api.sshCloseShell(tabId);
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Re-fit when tab becomes visible
  useEffect(() => {
    if (!active || !fitRef.current || !xtermRef.current) return;
    const t = setTimeout(() => {
      try {
        fitRef.current.fit();
        window.api.sshShellResize(tabId, xtermRef.current.cols, xtermRef.current.rows);
        xtermRef.current.focus();
      } catch (_) {}
    }, 80);
    return () => clearTimeout(t);
  }, [active, tabId]);

  // Theme updates
  useEffect(() => {
    if (xtermRef.current) xtermRef.current.options.theme = THEMES[theme];
  }, [theme]);

  // Font size updates
  useEffect(() => {
    if (!xtermRef.current || !fitRef.current) return;
    xtermRef.current.options.fontSize = fontSize;
    setTimeout(() => {
      try { fitRef.current.fit(); window.api.sshShellResize(tabId, xtermRef.current.cols, xtermRef.current.rows); } catch (_) {}
    }, 50);
  }, [fontSize, tabId]);

  return (
    <div
      style={{ display: active ? 'block' : 'none', position: 'absolute', inset: 8, overflow: 'hidden' }}
      ref={containerRef}
    />
  );
}

// ── Main exported component ───────────────────────────────────────────────────
export default function TerminalPage({ server }) {
  const [tabs, setTabs]               = useState([FIRST_TAB]);
  const [activeTabId, setActiveTabId] = useState(FIRST_TAB.id);
  const [fontSize, setFontSize]     = useState(14);
  const [activeTheme, setActiveTheme] = useState('Glyph Default');
  const [showSearch, setShowSearch] = useState(false);
  const [searchText, setSearchText] = useState('');
  const [quickCommands, setQuickCommands] = useState([]);
  const [showCmds, setShowCmds]     = useState(false);
  const [showThemes, setShowThemes] = useState(false);
  const [vaultSecrets, setVaultSecrets] = useState([]);
  const [showSecrets, setShowSecrets] = useState(false);

  // Per-pane search/refs map (tabId → { searchRef, xtermRef })
  const paneRefs = useRef({}); // tabId → { searchRef, xtermRef }

  // Initial activeTabId — sync when tabs change
  const activeTab = tabs.find(t => t.id === activeTabId) || tabs[0];

  // Listen for SSH shell output per tab
  useEffect(() => {
    const remove = window.api.onSshShellOutputTab((tabId, data) => {
      // We write data into the xterm via the per-tab container DOM
      // The TerminalPane handles its own listener via the terminal.onData — but
      // we need a way to push incoming data. We use a custom event bus.
      window.dispatchEvent(new CustomEvent(`__shell_data_${tabId}`, { detail: data }));
    });
    const removeClosed = window.api.onSshShellClosed((tabId) => {
      window.dispatchEvent(new CustomEvent(`__shell_closed_${tabId}`));
    });
    return () => { remove(); removeClosed(); };
  }, []);

  // Load quick commands for this server
  useEffect(() => {
    const key = server ? `glyph_commands_${server.id}` : 'glyph_commands';
    const saved = localStorage.getItem(key);
    if (saved) setQuickCommands(JSON.parse(saved).sort((a, b) => (b.uses || 0) - (a.uses || 0)));
  }, [server]);

  const addTab = () => {
    const tab = makeTab();
    setTabs(prev => [...prev, tab]);
    setActiveTabId(tab.id);
  };

  const closeTab = (e, id) => {
    e.stopPropagation();
    setTabs(prev => {
      const next = prev.filter(t => t.id !== id);
      if (activeTabId === id && next.length > 0) {
        setActiveTabId(next[next.length - 1].id);
      }
      return next;
    });
    // shell is closed by TerminalPane's cleanup
  };

  const execCommand = (cmdStr) => {
    if (cmdStr && activeTabId) window.api.sshShellData(activeTabId, cmdStr + '\n');
    setShowCmds(false);
  };

  const injectSecret = (id) => {
    window.api.injectSecret(id);
    setShowSecrets(false);
  };

  const loadSecrets = async () => {
    if (server?.id) {
      const list = await window.api.getSecrets(server.id);
      setVaultSecrets(list);
    }
  };

  // Per-tab search addon — stored in a Map via the global paneRefs
  const getSearchAddon = () => paneRefs.current[activeTabId]?.searchRef?.current;

  return (
    <div
      className="w-full h-full flex flex-col overflow-hidden relative"
      style={{ backgroundColor: THEMES[activeTheme].background }}
    >
      {/* Top Toolbar */}
      <div className="h-12 bg-dark-800/80 backdrop-blur-md border-b border-dark-700 flex items-center justify-between px-4 shrink-0 z-50">
        <div className="flex items-center gap-2">

          {/* Snippets */}
          <div className="relative">
            <button
              onClick={() => { setShowCmds(!showCmds); setShowThemes(false); setShowSecrets(false); }}
              className="px-3 py-1.5 bg-dark-700 hover:bg-brand-500 hover:text-white text-gray-300 rounded-md text-xs font-semibold flex items-center gap-2 transition-colors"
            >
              <Zap size={14} /> Snippets <ChevronDown size={14} />
            </button>
            {showCmds && (
              <div className="absolute top-full mt-2 left-0 w-64 bg-dark-800 border border-dark-600 rounded-lg shadow-xl overflow-hidden py-1 z-50">
                {quickCommands.length === 0
                  ? <div className="px-4 py-3 text-xs text-gray-400">No commands saved. Use the Commands tab!</div>
                  : quickCommands.map(c => (
                    <button key={c.id} onClick={() => execCommand(c.cmd)}
                      className="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-brand-500 hover:text-white transition-colors truncate">
                      {c.name}
                    </button>
                  ))
                }
              </div>
            )}
          </div>

          {/* Secrets */}
          <div className="relative ml-2">
            <button
              onClick={() => { setShowSecrets(!showSecrets); setShowCmds(false); setShowThemes(false); loadSecrets(); }}
              className="px-3 py-1.5 bg-dark-700 hover:bg-brand-500 hover:text-white text-gray-300 rounded-md text-xs font-semibold flex items-center gap-2 transition-colors"
            >
              <Lock size={14} /> Secrets <ChevronDown size={14} />
            </button>
            {showSecrets && (
              <div className="absolute top-full mt-2 left-0 w-64 bg-dark-800 border border-dark-600 rounded-lg shadow-xl overflow-hidden py-1 z-50">
                {vaultSecrets.length === 0
                  ? <div className="px-4 py-3 text-xs text-gray-400">Vault is empty. Add secrets in the Secrets tab!</div>
                  : vaultSecrets.map(s => (
                    <button key={s.id} onClick={() => injectSecret(s.id)}
                      className="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-brand-500 hover:text-white transition-colors flex items-center justify-between">
                      <span className="truncate">{s.name}</span>
                      <span className="text-[10px] uppercase bg-dark-900/50 px-1.5 py-0.5 rounded text-gray-400">Inject</span>
                    </button>
                  ))
                }
              </div>
            )}
          </div>

          {/* Theme */}
          <div className="relative ml-2">
            <button
              onClick={() => { setShowThemes(!showThemes); setShowCmds(false); setShowSecrets(false); }}
              className="px-3 py-1.5 bg-dark-700 hover:bg-dark-600 text-gray-300 rounded-md text-xs font-semibold flex items-center gap-2 transition-colors"
            >
              <Palette size={14} /> Theme <ChevronDown size={14} />
            </button>
            {showThemes && (
              <div className="absolute top-full mt-2 left-0 w-48 bg-dark-800 border border-dark-600 rounded-lg shadow-xl overflow-hidden py-1 z-50">
                {Object.keys(THEMES).map(t => (
                  <button key={t} onClick={() => { setActiveTheme(t); setShowThemes(false); }}
                    className={`w-full text-left px-4 py-2 text-sm transition-colors ${activeTheme === t ? 'bg-brand-500 text-white' : 'text-gray-300 hover:bg-dark-700'}`}>
                    {t}
                  </button>
                ))}
              </div>
            )}
          </div>

          <div className="w-px h-6 bg-dark-700 mx-2" />

          <button onClick={() => setFontSize(f => Math.max(8, f - 1))}
            className="p-1.5 text-gray-400 hover:text-gray-200 hover:bg-dark-700 rounded-md transition-colors" title="Decrease Font Size">
            <Minus size={16} />
          </button>
          <span className="text-xs font-mono text-gray-400 w-6 text-center">{fontSize}</span>
          <button onClick={() => setFontSize(f => Math.min(32, f + 1))}
            className="p-1.5 text-gray-400 hover:text-gray-200 hover:bg-dark-700 rounded-md transition-colors" title="Increase Font Size">
            <Plus size={16} />
          </button>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => {
              if (showSearch) { setShowSearch(false); getSearchAddon()?.clearDecorations(); }
              else setShowSearch(true);
            }}
            className={`p-2 rounded-md transition-colors ${showSearch ? 'bg-brand-500 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-dark-700'}`}
            title="Search (Ctrl+F)"
          >
            <Search size={16} />
          </button>

          <div className="w-px h-6 bg-dark-700 mx-1" />

          <button
            onClick={() => {
              // clear active terminal
              window.dispatchEvent(new CustomEvent(`__shell_clear_${activeTabId}`));
            }}
            className="p-2 text-gray-400 hover:text-red-400 hover:bg-dark-700 rounded-md transition-colors" title="Clear Terminal"
          >
            <Trash2 size={16} />
          </button>
        </div>
      </div>

      {/* Tab bar */}
      <div className="flex items-center gap-1 px-3 py-1.5 bg-dark-900/70 border-b border-dark-700 shrink-0 overflow-x-auto custom-scrollbar">
        {tabs.map(tab => (
          <div
            key={tab.id}
            onClick={() => setActiveTabId(tab.id)}
            className={`flex items-center gap-1.5 px-3 py-1 rounded-md cursor-pointer text-xs font-medium transition-colors whitespace-nowrap group ${
              activeTabId === tab.id
                ? 'bg-brand-500/20 text-brand-300 border border-brand-500/40'
                : 'text-gray-400 hover:bg-dark-700 hover:text-gray-200'
            }`}
          >
            <span className="max-w-[120px] truncate">{tab.title}</span>
            {tabs.length > 1 && (
              <button
                onClick={(e) => closeTab(e, tab.id)}
                className="opacity-0 group-hover:opacity-100 hover:text-white rounded transition-all p-0.5"
              >
                <X size={11} />
              </button>
            )}
          </div>
        ))}
        <button
          onClick={addTab}
          className="ml-1 p-1.5 text-gray-500 hover:text-brand-400 hover:bg-dark-700 rounded-md transition-colors shrink-0"
          title="New Terminal Tab"
        >
          <SquarePlus size={16} />
        </button>
      </div>

      {/* Search Bar */}
      {showSearch && (
        <div className="absolute top-14 right-4 bg-dark-800 border border-dark-600 shadow-xl rounded-lg flex items-center p-1.5 z-20">
          <input
            autoFocus
            value={searchText}
            onChange={(e) => {
              setSearchText(e.target.value);
              getSearchAddon()?.findNext(e.target.value);
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                if (e.shiftKey) getSearchAddon()?.findPrevious(searchText);
                else getSearchAddon()?.findNext(searchText);
              }
            }}
            className="bg-transparent border-none focus:outline-none text-sm text-gray-200 px-2 w-48"
            placeholder="Find..."
          />
          <button onClick={() => getSearchAddon()?.findPrevious(searchText)}
            className="p-1.5 text-gray-400 hover:text-gray-200 hover:bg-dark-700 rounded-md"><ArrowUp size={14} /></button>
          <button onClick={() => getSearchAddon()?.findNext(searchText)}
            className="p-1.5 text-gray-400 hover:text-gray-200 hover:bg-dark-700 rounded-md"><ArrowDown size={14} /></button>
          <div className="w-px h-4 bg-dark-700 mx-1" />
          <button onClick={() => { setShowSearch(false); getSearchAddon()?.clearDecorations(); setSearchText(''); }}
            className="p-1.5 text-gray-400 hover:text-gray-200 hover:bg-dark-700 rounded-md"><X size={14} /></button>
        </div>
      )}

      {/* Close dropdowns on outside click */}
      {(showCmds || showThemes || showSecrets) && (
        <div className="absolute inset-0 z-40"
          onClick={() => { setShowCmds(false); setShowThemes(false); setShowSecrets(false); }} />
      )}

      {/* Terminal Panes — all mounted, CSS hides inactive ones */}
      <div className="flex-1 w-full relative z-0">
        {tabs.map(tab => (
          <TerminalPaneWrapper
            key={tab.id}
            tabId={tab.id}
            server={server}
            theme={activeTheme}
            fontSize={fontSize}
            active={activeTabId === tab.id}
            onRegister={(refs) => { paneRefs.current[tab.id] = refs; }}
            onUnregister={() => { delete paneRefs.current[tab.id]; }}
          />
        ))}
      </div>
    </div>
  );
}

// ── Wrapper that exposes xterm/searchAddon refs upward ────────────────────────
function TerminalPaneWrapper({ tabId, server, theme, fontSize, active, onRegister, onUnregister }) {
  const containerRef = useRef(null);
  const xtermRef     = useRef(null);
  const fitRef       = useRef(null);
  const searchRef    = useRef(null);
  const disposedRef  = useRef(false);
  const openedRef    = useRef(false);
  const cleanupRef   = useRef(null);
  // Only initialize xterm after the tab is first visible — prevents
  // the "Cannot read properties of undefined (reading 'dimensions')"
  // crash that occurs when xterm.open() is called on a display:none element.
  const [everActive, setEverActive] = useState(active);

  useEffect(() => {
    if (active && !everActive) setEverActive(true);
  }, [active]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    onRegister({ xtermRef, searchRef });
    return () => onUnregister();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Init xterm — only after first visible
  useEffect(() => {
    if (!everActive) return;
    if (!containerRef.current || xtermRef.current) return;
    disposedRef.current = false;

    // Defer initialization by one tick so the browser's layout engine
    // has a chance to calculate the container's width/height after switching
    // from display:none to display:block. This prevents xterm from crashing
    // while trying to fit into a 0x0 box.
    const initTimer = setTimeout(() => {
      if (disposedRef.current || !containerRef.current || xtermRef.current) return;

      const term = new Terminal({
        cursorBlink: true,
        theme: THEMES[theme],
        fontFamily: '"Fira Code", monospace',
        fontSize,
      });

      const fit    = new FitAddon();
      const search = new SearchAddon();
      term.loadAddon(fit);
      term.loadAddon(search);
      term.open(containerRef.current);

      xtermRef.current  = term;
      fitRef.current    = fit;
      searchRef.current = search;

      let fitTimer = null;
      const safefit = () => {
        if (disposedRef.current) return;
        try {
          if (!containerRef.current || containerRef.current.offsetParent === null) return;
          if (containerRef.current.clientWidth > 0 && containerRef.current.clientHeight > 0) {
            fit.fit();
            window.api.sshShellResize(tabId, term.cols, term.rows);
          }
        } catch (_) {}
      };
      const debouncedFit = () => {
        clearTimeout(fitTimer);
        fitTimer = setTimeout(safefit, 80);
      };

      const resizeObserver = new ResizeObserver(debouncedFit);
      resizeObserver.observe(containerRef.current);
      window.addEventListener('resize', debouncedFit);
      fitTimer = setTimeout(safefit, 250);

      // Input → SSH
      term.onData(data => {
        if (!disposedRef.current) window.api.sshShellData(tabId, data);
      });

      // Copy / Paste
      term.attachCustomKeyEventHandler((e) => {
        if (e.type !== 'keydown') return true;
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'c' && term.hasSelection()) {
          navigator.clipboard.writeText(term.getSelection());
          return false;
        }
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'v') return false;
        return true;
      });

      // SSH output → xterm
      const onData = (e) => { if (!disposedRef.current && xtermRef.current) term.write(e.detail); };
      window.addEventListener(`__shell_data_${tabId}`, onData);

      // Shell closed externally
      const onClosed = () => { if (!disposedRef.current) term.write('\r\n\x1b[33m[Session closed]\x1b[0m\r\n'); };
      window.addEventListener(`__shell_closed_${tabId}`, onClosed);

      // Clear command
      const onClear = () => { if (!disposedRef.current && xtermRef.current) term.clear(); };
      window.addEventListener(`__shell_clear_${tabId}`, onClear);

      // Open SSH shell for this tab
      window.api.sshOpenShell(tabId).then(() => {
        openedRef.current = true;
        debouncedFit();
      }).catch(err => {
        term.write(`\r\n\x1b[31mFailed to open shell: ${err.message}\x1b[0m\r\n`);
      });

      cleanupRef.current = () => {
        clearTimeout(fitTimer);
        resizeObserver.disconnect();
        window.removeEventListener('resize', debouncedFit);
        window.removeEventListener(`__shell_data_${tabId}`, onData);
        window.removeEventListener(`__shell_closed_${tabId}`, onClosed);
        window.removeEventListener(`__shell_clear_${tabId}`, onClear);
        term.dispose();
      };
    }, 10);

    return () => {
      disposedRef.current = true;
      clearTimeout(initTimer);
      if (cleanupRef.current) {
        cleanupRef.current();
        cleanupRef.current = null;
      }
      xtermRef.current = null;
      if (openedRef.current) window.api.sshCloseShell(tabId);
    };
  }, [everActive]); // re-triggers once when everActive first becomes true

  // Refit when becoming active
  useEffect(() => {
    if (!active || !fitRef.current || !xtermRef.current) return;
    const t = setTimeout(() => {
      try {
        fitRef.current.fit();
        window.api.sshShellResize(tabId, xtermRef.current.cols, xtermRef.current.rows);
        xtermRef.current.focus();
      } catch (_) {}
    }, 80);
    return () => clearTimeout(t);
  }, [active, tabId]);

  // Theme updates
  useEffect(() => {
    if (xtermRef.current) xtermRef.current.options.theme = THEMES[theme];
  }, [theme]);

  // Font size updates
  useEffect(() => {
    if (!xtermRef.current || !fitRef.current) return;
    xtermRef.current.options.fontSize = fontSize;
    setTimeout(() => {
      try { fitRef.current.fit(); window.api.sshShellResize(tabId, xtermRef.current.cols, xtermRef.current.rows); } catch (_) {}
    }, 50);
  }, [fontSize, tabId]);

  return (
    <div
      style={{
        display: active ? 'block' : 'none',
        position: 'absolute',
        inset: 8,
        overflow: 'hidden',
      }}
      ref={containerRef}
    />
  );
}
