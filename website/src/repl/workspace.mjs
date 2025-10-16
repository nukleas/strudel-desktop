import { atom } from 'nanostores';

// Tauri v2 API - lazy loaded
let _invoke, _open, _save;
let _tauriInitialized = false;
let _isTauri = false;

async function ensureTauriAPIs() {
  if (_tauriInitialized) return _isTauri;

  _tauriInitialized = true;

  if (typeof window === 'undefined') {
    _isTauri = false;
    return false;
  }

  try {
    const [tauriCore, tauriDialog] = await Promise.all([
      import('@tauri-apps/api/core'),
      import('@tauri-apps/plugin-dialog')
    ]);
    _invoke = tauriCore.invoke;
    _open = tauriDialog.open;
    _save = tauriDialog.save;
    _isTauri = true;
    return true;
  } catch (e) {
    // Not in Tauri environment
    console.error('Failed to initialize Tauri APIs:', e);
    _isTauri = false;
    return false;
  }
}

// Check if we're in Tauri context synchronously (best effort)
const isTauriContext = () => {
  if (_tauriInitialized) return _isTauri;
  // Check for Tauri-injected globals as a quick check
  return typeof window !== 'undefined' && (
    window.__TAURI_INTERNALS__ !== undefined ||
    window.__TAURI_INVOKE__ !== undefined
  );
};

// Helper to get APIs
const getInvoke = async () => {
  await ensureTauriAPIs();
  return _invoke;
};

const getDialogAPIs = async () => {
  await ensureTauriAPIs();
  return { open: _open, save: _save };
};

// Workspace state
export const workspaceDir = atom(null);
export const recentFiles = atom([]);
export const currentFilePath = atom(null);
export const hasUnsavedChanges = atom(false);
export const workspaceFiles = atom([]);

/**
 * Initialize workspace - load workspace directory and recent files
 */
export async function initWorkspace() {
  if (!isTauriContext()) return;

  const invoke = await getInvoke();
  if (!invoke) return;

  try {
    const dir = await invoke('get_workspace_dir');
    workspaceDir.set(dir);

    const recent = await invoke('get_recent_files');
    recentFiles.set(recent);

    await refreshWorkspaceFiles();

    // Load last opened file
    const lastOpened = await invoke('get_last_opened_file');
    if (lastOpened) {
      currentFilePath.set(lastOpened);
    }
  } catch (error) {
    console.error('Failed to initialize workspace:', error);
  }
}

/**
 * Refresh workspace files list
 */
export async function refreshWorkspaceFiles() {
  if (!isTauriContext()) return;

  const invoke = await getInvoke();
  if (!invoke) return;

  try {
    const files = await invoke('list_workspace_files');
    workspaceFiles.set(files);
  } catch (error) {
    console.error('Failed to list workspace files:', error);
  }
}

/**
 * Save code to file
 */
export async function saveCodeFile(path, content) {
  if (!isTauriContext()) {
    throw new Error('Tauri not available');
  }

  const invoke = await getInvoke();
  if (!invoke) {
    throw new Error('Tauri not available');
  }

  try {
    await invoke('save_code_file', { path, content });
    currentFilePath.set(path);
    hasUnsavedChanges.set(false);

    // Refresh recent files and workspace
    const recent = await invoke('get_recent_files');
    recentFiles.set(recent);
    await refreshWorkspaceFiles();

    return path;
  } catch (error) {
    console.error('Failed to save file:', error);
    throw error;
  }
}

/**
 * Load code from file
 */
export async function loadCodeFile(path) {
  if (!isTauriContext()) {
    throw new Error('Tauri not available');
  }

  const invoke = await getInvoke();
  if (!invoke) {
    throw new Error('Tauri not available');
  }

  try {
    const content = await invoke('load_code_file', { path });
    currentFilePath.set(path);
    hasUnsavedChanges.set(false);

    // Refresh recent files
    const recent = await invoke('get_recent_files');
    recentFiles.set(recent);

    return content;
  } catch (error) {
    console.error('Failed to load file:', error);
    throw error;
  }
}

/**
 * Open file dialog and load selected file
 */
export async function openFileDialog() {
  if (!isTauriContext()) {
    throw new Error('Tauri not available');
  }

  const { open } = await getDialogAPIs();
  if (!open) {
    throw new Error('Tauri dialog not available');
  }

  try {
    const dir = workspaceDir.get();
    const selected = await open({
      multiple: false,
      directory: false,
      defaultPath: dir,
      filters: [
        {
          name: 'Strudel Files',
          extensions: ['strudel', 'js', 'txt'],
        },
      ],
    });

    if (selected) {
      return await loadCodeFile(selected);
    }
    return null;
  } catch (error) {
    console.error('Failed to open file dialog:', error);
    throw error;
  }
}

/**
 * Save file dialog and save code
 */
export async function saveFileDialog(content, suggestedName = 'untitled.strudel') {
  if (!isTauriContext()) {
    throw new Error('Tauri not available');
  }

  const { save } = await getDialogAPIs();
  if (!save) {
    throw new Error('Tauri dialog not available');
  }

  try {
    const dir = workspaceDir.get();
    const selected = await save({
      defaultPath: dir ? `${dir}/${suggestedName}` : suggestedName,
      filters: [
        {
          name: 'Strudel Files',
          extensions: ['strudel'],
        },
      ],
    });

    if (selected) {
      // Ensure .strudel extension
      const path = selected.endsWith('.strudel') ? selected : `${selected}.strudel`;
      await saveCodeFile(path, content);
      return path;
    }
    return null;
  } catch (error) {
    console.error('Failed to save file dialog:', error);
    throw error;
  }
}

/**
 * Save current file (if path exists) or open save dialog
 */
export async function saveCurrentFile(content) {
  const path = currentFilePath.get();

  if (path) {
    return await saveCodeFile(path, content);
  } else {
    return await saveFileDialog(content);
  }
}

/**
 * Create new file in workspace
 */
export async function createNewFile(filename, content = '// New Strudel pattern\n\n') {
  if (!isTauriContext()) {
    throw new Error('Tauri not available');
  }

  const invoke = await getInvoke();
  if (!invoke) {
    throw new Error('Tauri not available');
  }

  try {
    // Ensure .strudel extension
    const name = filename.endsWith('.strudel') ? filename : `${filename}.strudel`;

    const path = await invoke('create_workspace_file', {
      filename: name,
      content,
    });

    currentFilePath.set(path);
    hasUnsavedChanges.set(false);

    // Refresh workspace and recent files
    await refreshWorkspaceFiles();
    const recent = await invoke('get_recent_files');
    recentFiles.set(recent);

    return path;
  } catch (error) {
    console.error('Failed to create new file:', error);
    throw error;
  }
}

/**
 * Delete file from workspace
 */
export async function deleteFile(path) {
  if (!isTauriContext()) {
    throw new Error('Tauri not available');
  }

  const invoke = await getInvoke();
  if (!invoke) {
    throw new Error('Tauri not available');
  }

  try {
    await invoke('delete_workspace_file', { path });

    // Clear current file if it was deleted
    if (currentFilePath.get() === path) {
      currentFilePath.set(null);
    }

    // Refresh workspace and recent files
    await refreshWorkspaceFiles();
    const recent = await invoke('get_recent_files');
    recentFiles.set(recent);
  } catch (error) {
    console.error('Failed to delete file:', error);
    throw error;
  }
}

/**
 * Set workspace directory
 */
export async function setWorkspaceDirectory(dir) {
  if (!isTauriContext()) {
    throw new Error('Tauri not available');
  }

  const invoke = await getInvoke();
  if (!invoke) {
    throw new Error('Tauri not available');
  }

  try {
    await invoke('set_workspace_dir', { dir });
    workspaceDir.set(dir);
    await refreshWorkspaceFiles();
  } catch (error) {
    console.error('Failed to set workspace directory:', error);
    throw error;
  }
}

/**
 * Pick workspace directory
 */
export async function pickWorkspaceDirectory() {
  if (!isTauriContext()) {
    throw new Error('Tauri not available');
  }

  const { open } = await getDialogAPIs();
  if (!open) {
    throw new Error('Tauri dialog not available');
  }

  try {
    const selected = await open({
      multiple: false,
      directory: true,
      defaultPath: workspaceDir.get(),
    });

    if (selected) {
      await setWorkspaceDirectory(selected);
      return selected;
    }
    return null;
  } catch (error) {
    console.error('Failed to pick workspace directory:', error);
    throw error;
  }
}

/**
 * Mark content as changed
 */
export function markAsChanged() {
  hasUnsavedChanges.set(true);
}

/**
 * Clear current file (for "new" action)
 */
export function clearCurrentFile() {
  currentFilePath.set(null);
  hasUnsavedChanges.set(false);
}

/**
 * Get current file name
 */
export function getCurrentFileName() {
  const path = currentFilePath.get();
  if (!path) return null;

  const parts = path.split('/');
  return parts[parts.length - 1];
}

/**
 * Format file size for display
 */
export function formatFileSize(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * Format relative time (e.g., "2m ago", "1h ago")
 */
export function formatRelativeTime(isoString) {
  const now = new Date();
  const then = new Date(isoString);
  const diffMs = now - then;
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffSec < 60) return 'just now';
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHour < 24) return `${diffHour}h ago`;
  if (diffDay < 7) return `${diffDay}d ago`;
  return then.toLocaleDateString();
}

// Export check for Tauri availability
export const isTauriAvailable = isTauriContext;
