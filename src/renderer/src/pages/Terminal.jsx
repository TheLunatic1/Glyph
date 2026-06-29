import React, { useEffect, useRef } from 'react';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';

// This component is always mounted in the DOM and shown/hidden via CSS
// to avoid the xterm re-initialization error when switching tabs.
export default function TerminalPage() {
  const terminalRef = useRef(null);
  const xtermRef = useRef(null);
  const fitAddonRef = useRef(null);
  const isDisposedRef = useRef(false);

  useEffect(() => {
    if (!terminalRef.current || xtermRef.current) return;

    isDisposedRef.current = false;

    const term = new Terminal({
      cursorBlink: true,
      theme: {
        background: '#0f111a',
        foreground: '#f3f4f6',
        cursor: '#6366f1',
        selectionBackground: '#818cf850',
      },
      fontFamily: '"Fira Code", monospace',
      fontSize: 14,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(terminalRef.current);

    xtermRef.current = term;
    fitAddonRef.current = fitAddon;

    const safefit = () => {
      if (isDisposedRef.current) return;
      try {
        if (!terminalRef.current || terminalRef.current.offsetParent === null) return;
        if (terminalRef.current.clientWidth > 0 && terminalRef.current.clientHeight > 0) {
          fitAddon.fit();
          window.api.sshShellResize(term.cols, term.rows);
        }
      } catch (e) {
        // ignore
      }
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

  return (
    <div className="w-full h-full bg-dark-900 flex flex-col p-4 overflow-hidden">
      <div ref={terminalRef} className="flex-1 min-h-0 w-full overflow-hidden" />
    </div>
  );
}
