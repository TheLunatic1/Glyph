import React, { useState, useCallback } from 'react';
import { Folder, File, ArrowUp, FolderOpen, RefreshCw, Edit3, X, Save } from 'lucide-react';
import Editor from '@monaco-editor/react';

export default function SFTP() {
  const [currentPath, setCurrentPath] = useState('/');
  const [files, setFiles] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [editingFile, setEditingFile] = useState(null);
  const [editorContent, setEditorContent] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const [isFetchingFile, setIsFetchingFile] = useState(false);
  const [fileError, setFileError] = useState(null); // Fix #7: styled error instead of alert()

  const loadDirectory = useCallback(async (path) => {
    setLoading(true);
    setError(null);
    try {
      const list = await window.api.sshSftpReaddir(path);
      list.sort((a, b) => {
        if (a.isDirectory && !b.isDirectory) return -1;
        if (!a.isDirectory && b.isDirectory) return 1;
        return a.filename.localeCompare(b.filename);
      });
      setFiles(list);
      setCurrentPath(path);
      setHasLoaded(true);
    } catch (err) {
      setError('SFTP is still initializing. Please wait a moment and try again.');
      console.warn('SFTP readdir error:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  const goUp = () => {
    if (currentPath === '/') return;
    const parts = currentPath.split('/').filter(Boolean);
    parts.pop();
    loadDirectory('/' + parts.join('/') || '/');
  };

  const handleNavigate = (file) => {
    if (file.isDirectory) {
      const newPath = currentPath === '/'
        ? `/${file.filename}`
        : `${currentPath}/${file.filename}`;
      loadDirectory(newPath);
    } else {
      openEditor(file);
    }
  };

  const openEditor = async (file) => {
    setIsFetchingFile(true);
    setFileError(null);
    const filePath = currentPath === '/' ? `/${file.filename}` : `${currentPath}/${file.filename}`;
    try {
      const content = await window.api.sshSftpReadFile(filePath);
      setEditorContent(content);
      setEditingFile({ ...file, path: filePath });
    } catch (err) {
      console.error('Failed to read file:', err);
      // Fix #7: styled error instead of alert()
      setFileError('Could not open file. It might be binary or you may lack read permissions.');
    } finally {
      setIsFetchingFile(false);
    }
  };

  const handleSaveFile = async (currentContent) => {
    if (!editingFile) return;
    setIsSaving(true);
    try {
      await window.api.sshSftpWriteFile(editingFile.path, currentContent);
      // Optional: show a quick success toast here
    } catch (err) {
      console.error('Failed to save file:', err);
      alert('Failed to save file!');
    } finally {
      setIsSaving(false);
    }
  };

  const handleEditorDidMount = (editor, monaco) => {
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
      handleSaveFile(editor.getValue());
    });
  };

  const formatSize = (bytes) => {
    if (!bytes || bytes === 0) return '—';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  };

  // Landing state — user hasn't opened any folder yet
  if (!hasLoaded) {
    return (
      <div className="p-8 h-full flex flex-col">
        <header className="mb-6">
          <h2 className="text-3xl font-bold text-gray-100">File Explorer</h2>
          <p className="text-gray-400 mt-1">Browse and manage files on the remote server</p>
        </header>
        <div className="flex-1 glass-panel flex flex-col items-center justify-center gap-4">
          <FolderOpen size={64} className="text-dark-600" />
          <h3 className="text-xl font-semibold text-gray-300">Click to open root directory</h3>
          <p className="text-gray-500 text-sm">SFTP session initializes on first use</p>
          <button
            onClick={() => loadDirectory('/')}
            disabled={loading}
            className="mt-2 px-6 py-2.5 bg-brand-500 hover:bg-brand-400 text-white font-semibold rounded-lg transition-colors flex items-center gap-2"
          >
            <FolderOpen size={18} />
            {loading ? 'Connecting...' : 'Browse /'}
          </button>
          {error && <p className="text-red-400 text-sm mt-2 max-w-sm text-center">{error}</p>}
        </div>
      </div>
    );
  }

  return (
    <div className="p-8 h-full flex flex-col">
      <header className="mb-6">
        <h2 className="text-3xl font-bold text-gray-100">File Explorer</h2>
        <div className="flex items-center gap-3 mt-4">
          <button onClick={goUp} disabled={currentPath === '/'} className="p-2 bg-dark-700 hover:bg-dark-600 rounded-lg transition-colors disabled:opacity-40">
            <ArrowUp size={20} />
          </button>
          <div className="glass-panel px-4 py-2 flex-1 text-brand-400 font-mono text-sm">
            {currentPath}
          </div>
          <button onClick={() => loadDirectory(currentPath)} disabled={loading} className="p-2 bg-dark-700 hover:bg-dark-600 rounded-lg transition-colors">
            <RefreshCw size={18} className={loading ? 'animate-spin' : ''} />
          </button>
        </div>
      </header>

      <div className="flex-1 glass-panel overflow-hidden flex flex-col">
        {error && (
          <div className="m-4 p-3 bg-red-500/10 border border-red-500/30 rounded-lg text-red-400 text-sm">
            {error}
          </div>
        )}
        {/* Fix #7: styled error for file open failures */}
        {fileError && (
          <div className="m-4 p-3 bg-orange-500/10 border border-orange-500/30 rounded-lg text-orange-400 text-sm flex items-center justify-between">
            <span>{fileError}</span>
            <button onClick={() => setFileError(null)} className="ml-4 text-orange-300 hover:text-white shrink-0">×</button>
          </div>
        )}
        {loading ? (
          <div className="flex-1 flex items-center justify-center text-gray-400 gap-2">
            <RefreshCw size={20} className="animate-spin" /> Loading...
          </div>
        ) : (
          <div className="overflow-y-auto p-4 flex flex-col gap-0.5">
            {/* Column headers */}
            <div className="flex items-center gap-3 px-3 pb-2 border-b border-dark-700 text-xs text-gray-500 uppercase tracking-wider">
              <span className="flex-1">Name</span>
              <span className="w-20 text-right">Size</span>
            </div>
            {files.map((file, idx) => (
              <div
                key={idx}
                onClick={() => handleNavigate(file)}
                className="flex items-center gap-3 p-3 rounded-lg transition-colors cursor-pointer hover:bg-dark-700/70 group"
              >
                {file.isDirectory
                  ? <Folder className="text-brand-400 shrink-0" size={20} />
                  : <File className="text-gray-500 shrink-0" size={20} />
                }
                <span className={`flex-1 truncate ${file.isDirectory ? 'text-gray-100 font-medium' : 'text-gray-300'}`}>
                  {file.filename}
                </span>
                <span className="text-gray-500 text-sm font-mono w-20 text-right">
                  {file.isDirectory ? '—' : formatSize(file.size)}
                </span>
              </div>
            ))}
            {files.length === 0 && (
              <div className="text-center text-gray-500 mt-10">Directory is empty</div>
            )}
          </div>
        )}
      </div>

      {/* Editor Overlay */}
      {isFetchingFile && (
        <div className="absolute inset-0 bg-dark-900/80 backdrop-blur-sm z-50 flex items-center justify-center">
          <div className="text-brand-400 flex flex-col items-center gap-4">
            <RefreshCw className="animate-spin" size={32} />
            <span className="font-medium text-lg">Fetching remote file...</span>
          </div>
        </div>
      )}

      {editingFile && (
        <div className="absolute inset-0 bg-dark-900 z-50 flex flex-col animation-slide-up">
          <div className="h-14 bg-dark-800/80 backdrop-blur-md border-b border-dark-700 flex items-center justify-between px-6 shrink-0 shadow-lg">
            <div className="flex items-center gap-3">
              <File className="text-brand-400" size={20} />
              <div>
                <h3 className="text-gray-100 font-semibold tracking-wide leading-tight">{editingFile.filename}</h3>
                <p className="text-xs text-gray-500 font-mono leading-tight">{editingFile.path}</p>
              </div>
            </div>
            <div className="flex items-center gap-3">
              <button 
                onClick={() => handleSaveFile(editorContent)}
                disabled={isSaving}
                className="px-4 py-2 bg-brand-500 hover:bg-brand-400 disabled:opacity-50 text-white text-sm font-semibold rounded-lg flex items-center gap-2 transition-colors shadow-lg shadow-brand-500/20"
              >
                {isSaving ? <RefreshCw className="animate-spin" size={16} /> : <Save size={16} />}
                {isSaving ? 'Saving...' : 'Save (Ctrl+S)'}
              </button>
              <div className="w-px h-6 bg-dark-700 mx-1"></div>
              <button 
                onClick={() => setEditingFile(null)}
                className="p-2 text-gray-400 hover:text-white hover:bg-dark-700 rounded-lg transition-colors"
                title="Close Editor"
              >
                <X size={20} />
              </button>
            </div>
          </div>
          <div className="flex-1 w-full bg-[#1e1e1e]">
            <Editor
              height="100%"
              theme="vs-dark"
              path={editingFile.filename}
              value={editorContent}
              onChange={(val) => setEditorContent(val)}
              onMount={handleEditorDidMount}
              options={{
                minimap: { enabled: true },
                fontSize: 14,
                fontFamily: '"Fira Code", monospace',
                wordWrap: 'on',
                smoothScrolling: true,
                cursorBlinking: 'smooth',
                padding: { top: 16 }
              }}
            />
          </div>
        </div>
      )}
    </div>
  );
}
