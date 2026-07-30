import React, { useState, useCallback, useEffect } from 'react';
import { Folder, File, ArrowUp, FolderOpen, RefreshCw, Edit3, X, Save, Upload, Download, Scissors, Copy, Trash2, FilePlus, FolderPlus, ClipboardPaste, MoreVertical, ChevronRight } from 'lucide-react';
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
  const [fileError, setFileError] = useState(null);

  // New state for Phase 2 features
  const [contextMenu, setContextMenu] = useState(null);
  const [clipboard, setClipboard] = useState(null);
  const [showRenameModal, setShowRenameModal] = useState(null);
  const [showDeleteModal, setShowDeleteModal] = useState(null);
  const [showCreateModal, setShowCreateModal] = useState(null);
  const [newName, setNewName] = useState('');
  const [isDragging, setIsDragging] = useState(false);
  const [dragCounter, setDragCounter] = useState(0);

  // Close context menu on click outside
  useEffect(() => {
    const handleClick = () => setContextMenu(null);
    window.addEventListener('click', handleClick);
    return () => window.removeEventListener('click', handleClick);
  }, []);

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

  const handleUpload = async () => {
    setLoading(true);
    try {
      const success = await window.api.sshSftpUpload(currentPath);
      if (success) {
        loadDirectory(currentPath);
      }
    } catch (err) {
      console.error('Upload failed:', err);
      setFileError('Upload failed: ' + err.message);
    } finally {
      setLoading(false);
    }
  };

  const handleDownload = async (file) => {
    setLoading(true);
    try {
      const remotePath = currentPath === '/' ? `/${file.filename}` : `${currentPath}/${file.filename}`;
      const success = await window.api.sshSftpDownload(remotePath, file.filename);
      if (success) {
        // Success
      }
    } catch (err) {
      console.error('Download failed:', err);
      setFileError('Download failed: ' + err.message);
    } finally {
      setLoading(false);
    }
  };

  const handleDragEnter = (e) => {
    e.preventDefault();
    e.stopPropagation();
    setDragCounter((prev) => {
      if (prev === 0) {
        setIsDragging(true);
      }
      return prev + 1;
    });
  };

  const handleDragLeave = (e) => {
    e.preventDefault();
    e.stopPropagation();
    
    // If the drag leaves the browser window entirely, reset completely.
    if (!e.relatedTarget) {
      setDragCounter(0);
      setIsDragging(false);
      return;
    }

    setDragCounter((prev) => {
      const newCounter = prev - 1;
      if (newCounter === 0) {
        setIsDragging(false);
      }
      return newCounter;
    });
  };

  const handleDragOver = (e) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleDrop = async (e) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
    setDragCounter(0);

    const droppedFiles = e.dataTransfer.files;
    if (!droppedFiles || droppedFiles.length === 0) return;

    const localPaths = Array.from(droppedFiles).map(f => f.path).filter(Boolean);
    if (localPaths.length === 0) return;

    setLoading(true);
    try {
      await window.api.sshSftpUploadDropped(localPaths, currentPath);
      loadDirectory(currentPath);
    } catch (err) {
      console.error('Drop upload failed:', err);
      setFileError('Upload failed: ' + err.message);
    } finally {
      setLoading(false);
    }
  };

  const handleContextMenu = (e, file) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      file,
    });
  };

  const handleCopy = () => {
    setClipboard({ action: 'copy', file: contextMenu.file, sourcePath: currentPath });
    setContextMenu(null);
  };

  const handleCut = () => {
    setClipboard({ action: 'cut', file: contextMenu.file, sourcePath: currentPath });
    setContextMenu(null);
  };

  const handlePaste = async () => {
    if (!clipboard) return;
    setLoading(true);
    try {
      const source = clipboard.sourcePath === '/' ? `/${clipboard.file.filename}` : `${clipboard.sourcePath}/${clipboard.file.filename}`;
      const dest = currentPath === '/' ? `/${clipboard.file.filename}` : `${currentPath}/${clipboard.file.filename}`;
      
      const cmd = clipboard.action === 'copy' ? `cp -r "${source}" "${dest}"` : `mv "${source}" "${dest}"`;
      await window.api.sshExec(cmd);
      
      if (clipboard.action === 'cut') {
        setClipboard(null);
      }
      loadDirectory(currentPath);
    } catch (err) {
      setFileError('Paste failed: ' + err.message);
    } finally {
      setLoading(false);
    }
  };

  const executeDelete = async () => {
    if (!showDeleteModal) return;
    setLoading(true);
    try {
      const target = currentPath === '/' ? `/${showDeleteModal.filename}` : `${currentPath}/${showDeleteModal.filename}`;
      await window.api.sshExec(`rm -rf "${target}"`);
      setShowDeleteModal(null);
      loadDirectory(currentPath);
    } catch (err) {
      setFileError('Delete failed: ' + err.message);
    } finally {
      setLoading(false);
    }
  };

  const executeRename = async () => {
    if (!showRenameModal || !newName) return;
    setLoading(true);
    try {
      const target = currentPath === '/' ? `/${showRenameModal.filename}` : `${currentPath}/${showRenameModal.filename}`;
      const newTarget = currentPath === '/' ? `/${newName}` : `${currentPath}/${newName}`;
      await window.api.sshExec(`mv "${target}" "${newTarget}"`);
      setShowRenameModal(null);
      setNewName('');
      loadDirectory(currentPath);
    } catch (err) {
      setFileError('Rename failed: ' + err.message);
    } finally {
      setLoading(false);
    }
  };

  const executeCreate = async () => {
    if (!showCreateModal || !newName) return;
    setLoading(true);
    try {
      const target = currentPath === '/' ? `/${newName}` : `${currentPath}/${newName}`;
      const cmd = showCreateModal.type === 'folder' ? `mkdir "${target}"` : `touch "${target}"`;
      await window.api.sshExec(cmd);
      setShowCreateModal(null);
      setNewName('');
      loadDirectory(currentPath);
    } catch (err) {
      setFileError('Create failed: ' + err.message);
    } finally {
      setLoading(false);
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
        <div className="flex items-center gap-2 mt-4">
          <button onClick={goUp} disabled={currentPath === '/'} className="p-2 bg-dark-700 hover:bg-dark-600 rounded-lg transition-colors disabled:opacity-40 shrink-0">
            <ArrowUp size={20} />
          </button>
          <div className="glass-panel px-3 py-1.5 flex-1 flex items-center overflow-x-auto custom-scrollbar gap-1 text-sm font-mono text-gray-300">
            <button 
              onClick={() => loadDirectory('/')}
              className="px-2 py-1 rounded hover:bg-dark-600 hover:text-white transition-colors"
            >
              /
            </button>
            {currentPath.split('/').filter(Boolean).map((part, index, arr) => {
              const path = '/' + arr.slice(0, index + 1).join('/');
              return (
                <React.Fragment key={path}>
                  <ChevronRight size={14} className="text-gray-600 shrink-0" />
                  <button 
                    onClick={() => loadDirectory(path)}
                    className="px-2 py-1 rounded hover:bg-dark-600 hover:text-brand-400 transition-colors shrink-0"
                  >
                    {part}
                  </button>
                </React.Fragment>
              );
            })}
          </div>
          <div className="flex items-center gap-1.5 shrink-0 pl-2">
            {clipboard && (
              <button onClick={handlePaste} disabled={loading} className="p-2 bg-brand-500/20 text-brand-400 hover:bg-brand-500 hover:text-white rounded-lg transition-colors shadow-lg flex items-center gap-2 px-3" title={`Paste ${clipboard.file.filename}`}>
                <ClipboardPaste size={18} />
                <span className="text-xs font-semibold uppercase tracking-wider">{clipboard.action}</span>
              </button>
            )}
            <button onClick={() => setShowCreateModal({ type: 'folder' })} disabled={loading} className="p-2 bg-dark-700 hover:bg-brand-500 hover:text-white rounded-lg transition-colors" title="New Folder">
              <FolderPlus size={18} />
            </button>
            <button onClick={() => setShowCreateModal({ type: 'file' })} disabled={loading} className="p-2 bg-dark-700 hover:bg-brand-500 hover:text-white rounded-lg transition-colors" title="New File">
              <FilePlus size={18} />
            </button>
            <button onClick={() => handleUpload()} disabled={loading} className="p-2 bg-dark-700 hover:bg-brand-500 hover:text-white rounded-lg transition-colors shadow-lg" title="Upload File to Current Directory">
              <Upload size={18} />
            </button>
            <button onClick={() => loadDirectory(currentPath)} disabled={loading} className="p-2 bg-dark-700 hover:bg-dark-600 rounded-lg transition-colors">
              <RefreshCw size={18} className={loading ? 'animate-spin' : ''} />
            </button>
          </div>
        </div>
      </header>

      <div 
        className="flex-1 glass-panel overflow-hidden flex flex-col relative"
        onDragEnter={handleDragEnter}
        onDragLeave={handleDragLeave}
        onDragOver={handleDragOver}
        onDrop={handleDrop}
      >
        {isDragging && (
          <div className="absolute inset-0 bg-brand-500/20 backdrop-blur-sm z-40 border-2 border-brand-500 border-dashed rounded-xl flex items-center justify-center pointer-events-none">
            <div className="bg-dark-800/90 p-6 rounded-2xl flex flex-col items-center gap-4 shadow-2xl pointer-events-none">
              <Upload size={48} className="text-brand-400 animate-bounce" />
              <h3 className="text-2xl font-bold text-gray-100">Drop files to upload</h3>
              <p className="text-gray-400">Uploading to {currentPath}</p>
            </div>
          </div>
        )}
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
                onContextMenu={(e) => handleContextMenu(e, file)}
                className="flex items-center gap-3 p-3 rounded-lg transition-colors cursor-pointer hover:bg-dark-700/70 group"
              >
                {file.isDirectory
                  ? <Folder className="text-brand-400 shrink-0" size={20} />
                  : <File className="text-gray-500 shrink-0" size={20} />
                }
                <span className={`flex-1 truncate ${file.isDirectory ? 'text-gray-100 font-medium' : 'text-gray-300'}`}>
                  {file.filename}
                </span>

                {/* Download Button */}
                <div className="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                  {!file.isDirectory && (
                    <button 
                      onClick={(e) => { e.stopPropagation(); handleDownload(file); }}
                      className="p-1.5 text-gray-400 hover:text-white bg-dark-700 hover:bg-brand-500 rounded transition-colors shadow-lg"
                      title="Download File"
                    >
                      <Download size={16} />
                    </button>
                  )}
                </div>

                <span className="text-gray-500 text-sm font-mono w-24 text-right shrink-0">
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

      {/* Context Menu */}
      {contextMenu && (
        <div 
          className="fixed z-50 bg-dark-800 border border-dark-700 shadow-2xl rounded-lg py-1.5 w-48 text-sm text-gray-300 animation-slide-up"
          style={{ top: contextMenu.y, left: contextMenu.x }}
          onClick={(e) => e.stopPropagation()}
        >
          <div className="px-3 py-1.5 border-b border-dark-700 mb-1 text-gray-400 font-medium truncate">
            {contextMenu.file.filename}
          </div>
          <button onClick={handleCopy} className="w-full text-left px-4 py-2 hover:bg-dark-700 hover:text-white flex items-center gap-2 transition-colors">
            <Copy size={16} /> Copy
          </button>
          <button onClick={handleCut} className="w-full text-left px-4 py-2 hover:bg-dark-700 hover:text-white flex items-center gap-2 transition-colors">
            <Scissors size={16} /> Cut
          </button>
          <button onClick={() => { setShowRenameModal(contextMenu.file); setNewName(contextMenu.file.filename); setContextMenu(null); }} className="w-full text-left px-4 py-2 hover:bg-dark-700 hover:text-white flex items-center gap-2 transition-colors">
            <Edit3 size={16} /> Rename
          </button>
          {!contextMenu.file.isDirectory && (
            <button onClick={() => { handleDownload(contextMenu.file); setContextMenu(null); }} className="w-full text-left px-4 py-2 hover:bg-dark-700 hover:text-white flex items-center gap-2 transition-colors">
              <Download size={16} /> Download
            </button>
          )}
          <div className="border-t border-dark-700 my-1"></div>
          <button onClick={() => { setShowDeleteModal(contextMenu.file); setContextMenu(null); }} className="w-full text-left px-4 py-2 hover:bg-red-500/20 text-red-400 hover:text-red-300 flex items-center gap-2 transition-colors">
            <Trash2 size={16} /> Delete
          </button>
        </div>
      )}

      {/* Rename Modal */}
      {showRenameModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-dark-900 border border-dark-700 w-full max-w-sm rounded-2xl shadow-2xl overflow-hidden flex flex-col">
            <div className="flex items-center justify-between p-4 border-b border-dark-800 bg-dark-800/30">
              <h3 className="text-gray-100 font-semibold">Rename</h3>
              <button onClick={() => setShowRenameModal(null)} className="p-2 text-gray-500 hover:text-gray-200 transition-colors">
                <X size={20}/>
              </button>
            </div>
            <div className="p-5 flex flex-col gap-4">
              <input
                type="text"
                autoFocus
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && executeRename()}
                className="w-full bg-dark-800 border border-dark-700 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-brand-500 transition-colors"
              />
              <div className="flex justify-end gap-2 mt-2">
                <button onClick={() => setShowRenameModal(null)} className="px-4 py-2 bg-dark-800 hover:bg-dark-700 text-gray-300 rounded-lg transition-colors font-medium text-sm">Cancel</button>
                <button onClick={executeRename} className="px-4 py-2 bg-brand-500 hover:bg-brand-400 text-white rounded-lg transition-colors font-medium text-sm shadow-lg shadow-brand-500/20">Save</button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Create Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-dark-900 border border-dark-700 w-full max-w-sm rounded-2xl shadow-2xl overflow-hidden flex flex-col">
            <div className="flex items-center justify-between p-4 border-b border-dark-800 bg-dark-800/30">
              <h3 className="text-gray-100 font-semibold">New {showCreateModal.type === 'folder' ? 'Folder' : 'File'}</h3>
              <button onClick={() => setShowCreateModal(null)} className="p-2 text-gray-500 hover:text-gray-200 transition-colors">
                <X size={20}/>
              </button>
            </div>
            <div className="p-5 flex flex-col gap-4">
              <input
                type="text"
                autoFocus
                placeholder={showCreateModal.type === 'folder' ? 'Folder Name' : 'File Name'}
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && executeCreate()}
                className="w-full bg-dark-800 border border-dark-700 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-brand-500 transition-colors"
              />
              <div className="flex justify-end gap-2 mt-2">
                <button onClick={() => setShowCreateModal(null)} className="px-4 py-2 bg-dark-800 hover:bg-dark-700 text-gray-300 rounded-lg transition-colors font-medium text-sm">Cancel</button>
                <button onClick={executeCreate} className="px-4 py-2 bg-brand-500 hover:bg-brand-400 text-white rounded-lg transition-colors font-medium text-sm shadow-lg shadow-brand-500/20">Create</button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Delete Modal */}
      {showDeleteModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="bg-dark-900 border border-dark-700 w-full max-w-sm rounded-2xl shadow-2xl overflow-hidden flex flex-col">
            <div className="flex items-center justify-between p-4 border-b border-dark-800 bg-dark-800/30">
              <div className="flex items-center gap-3">
                <Trash2 className="text-red-400" size={20} />
                <h3 className="text-gray-100 font-semibold">Delete {showDeleteModal.isDirectory ? 'Folder' : 'File'}</h3>
              </div>
              <button onClick={() => setShowDeleteModal(null)} className="p-2 text-gray-500 hover:text-gray-200 transition-colors">
                <X size={20}/>
              </button>
            </div>
            <div className="p-5 flex flex-col gap-4">
              <p className="text-sm text-gray-300">Are you sure you want to delete <span className="font-semibold text-white">{showDeleteModal.filename}</span>? This action cannot be undone.</p>
              <div className="flex justify-end gap-2 mt-2">
                <button onClick={() => setShowDeleteModal(null)} className="px-4 py-2 bg-dark-800 hover:bg-dark-700 text-gray-300 rounded-lg transition-colors font-medium text-sm">Cancel</button>
                <button onClick={executeDelete} className="px-4 py-2 bg-red-500 hover:bg-red-600 text-white rounded-lg transition-colors font-medium text-sm shadow-lg shadow-red-500/20">Delete</button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
