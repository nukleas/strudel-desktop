import { useState } from 'react';
import cx from '@src/cx.mjs';
import { ActionButton } from '../button/action-button.jsx';
import { XMarkIcon } from '@heroicons/react/16/solid';

const TAURI = typeof window !== 'undefined' && window.__TAURI_INTERNALS__;

// Lazy-load Tauri APIs
let _open, _invoke;

async function getDialogOpen() {
  if (!_open && TAURI) {
    const dialog = await import('@tauri-apps/plugin-dialog');
    _open = dialog.open;
  }
  return _open;
}

async function getInvoke() {
  if (!_invoke && TAURI) {
    const tauriCore = await import('@tauri-apps/api/core');
    _invoke = tauriCore.invoke;
  }
  return _invoke;
}

export function MidiImportButton({ context }) {
  const [showModal, setShowModal] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState(null);
  const [preview, setPreview] = useState(null);
  const [options, setOptions] = useState({
    compact: true,
    tempoScale: 1.0,
    notesPerBar: 64,
    barLimit: 0,
    tabSize: 2,
  });

  if (!TAURI) {
    return null; // Hide button in web version
  }

  const handleSelectFile = async () => {
    setError(null);
    setIsLoading(true);

    try {
      const open = await getDialogOpen();
      const invoke = await getInvoke();

      if (!open || !invoke) {
        throw new Error('Tauri APIs not available');
      }

      // Open file picker
      const filePath = await open({
        title: 'Import MIDI File',
        filters: [
          {
            name: 'MIDI Files',
            extensions: ['mid', 'midi'],
          },
        ],
      });

      if (!filePath) {
        setIsLoading(false);
        return; // User cancelled
      }

      // Convert MIDI to Strudel code
      const strudelCode = await invoke('import_midi_file', {
        filePath,
        options,
      });

      setPreview(strudelCode);
    } catch (err) {
      console.error('MIDI import failed:', err);
      setError(err.toString());
    } finally {
      setIsLoading(false);
    }
  };

  const handleInsert = () => {
    if (!preview || !context) return;

    // Create a pattern object similar to userPattern structure
    const patternData = {
      id: Date.now().toString(),
      code: preview,
      created_at: new Date().toISOString(),
      collection: 'user',
    };

    // Update the editor with the converted code
    context.handleUpdate(patternData, false);

    // Close modal and reset state
    setShowModal(false);
    setPreview(null);
    setError(null);
  };

  const handleCancel = () => {
    setShowModal(false);
    setPreview(null);
    setError(null);
  };

  return (
    <>
      <ActionButton label="import midi" onClick={() => setShowModal(true)} />

      {showModal && (
        <div
          className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
          onClick={(e) => {
            if (e.target === e.currentTarget) {
              handleCancel();
            }
          }}
        >
          <div className="bg-background border border-lineHighlight rounded-lg shadow-xl max-w-2xl w-full max-h-[80vh] overflow-hidden flex flex-col">
            {/* Header */}
            <div className="flex justify-between items-center p-4 border-b border-lineHighlight">
              <h3 className="text-lg font-bold text-foreground">Import MIDI File</h3>
              <button
                onClick={handleCancel}
                className="text-foreground hover:opacity-50 cursor-pointer"
                aria-label="Close"
              >
                <XMarkIcon className="w-6 h-6" />
              </button>
            </div>

            {/* Content */}
            <div className="flex-grow overflow-auto p-4 space-y-4">
              {/* Options Form */}
              {!preview && (
                <div className="space-y-3">
                  <div className="flex items-center gap-2">
                    <label className="flex items-center gap-2 text-sm text-foreground">
                      <input
                        type="checkbox"
                        checked={options.compact}
                        onChange={(e) => setOptions({ ...options, compact: e.target.checked })}
                        className="w-4 h-4"
                      />
                      Compact mode (use ! for repetitions)
                    </label>
                  </div>

                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1">
                      <label className="text-sm text-foreground">Tempo scale</label>
                      <input
                        type="number"
                        step="0.1"
                        min="0.1"
                        max="4.0"
                        value={options.tempoScale}
                        onChange={(e) => setOptions({ ...options, tempoScale: parseFloat(e.target.value) || 1.0 })}
                        className="px-3 py-2 bg-lineBackground text-foreground border border-lineHighlight rounded"
                      />
                      <span className="text-xs text-foreground opacity-50">1.0 = original tempo</span>
                    </div>

                    <div className="flex flex-col gap-1">
                      <label className="text-sm text-foreground">Notes per bar</label>
                      <input
                        type="number"
                        step="1"
                        min="16"
                        max="128"
                        value={options.notesPerBar}
                        onChange={(e) => setOptions({ ...options, notesPerBar: parseInt(e.target.value) || 64 })}
                        className="px-3 py-2 bg-lineBackground text-foreground border border-lineHighlight rounded"
                      />
                      <span className="text-xs text-foreground opacity-50">Higher = more precise</span>
                    </div>

                    <div className="flex flex-col gap-1">
                      <label className="text-sm text-foreground">Bar limit</label>
                      <input
                        type="number"
                        step="1"
                        min="0"
                        value={options.barLimit}
                        onChange={(e) => setOptions({ ...options, barLimit: parseInt(e.target.value) || 0 })}
                        className="px-3 py-2 bg-lineBackground text-foreground border border-lineHighlight rounded"
                      />
                      <span className="text-xs text-foreground opacity-50">0 = unlimited</span>
                    </div>

                    <div className="flex flex-col gap-1">
                      <label className="text-sm text-foreground">Tab size</label>
                      <input
                        type="number"
                        step="1"
                        min="2"
                        max="8"
                        value={options.tabSize}
                        onChange={(e) => setOptions({ ...options, tabSize: parseInt(e.target.value) || 2 })}
                        className="px-3 py-2 bg-lineBackground text-foreground border border-lineHighlight rounded"
                      />
                      <span className="text-xs text-foreground opacity-50">Indentation</span>
                    </div>
                  </div>
                </div>
              )}

              {/* Error Display */}
              {error && (
                <div className="p-3 bg-red-900/20 border border-red-500 rounded text-sm text-red-400">
                  <strong>Error:</strong> {error}
                </div>
              )}

              {/* Preview Display */}
              {preview && (
                <div className="space-y-2">
                  <label className="text-sm font-bold text-foreground">Preview:</label>
                  <pre className="bg-lineBackground p-3 rounded overflow-auto max-h-96 text-xs font-mono text-foreground">
                    {preview}
                  </pre>
                  <p className="text-xs text-foreground opacity-50">
                    {preview.split('\n').length} lines • {preview.length} characters
                  </p>
                </div>
              )}

              {/* Loading State */}
              {isLoading && (
                <div className="flex items-center justify-center py-8">
                  <div className="text-foreground animate-pulse">Converting MIDI file...</div>
                </div>
              )}
            </div>

            {/* Footer Actions */}
            <div className="flex justify-end gap-2 p-4 border-t border-lineHighlight">
              {!preview && !isLoading && (
                <button
                  onClick={handleSelectFile}
                  className="px-4 py-2 bg-[var(--cyan-400)] text-background rounded hover:bg-[var(--cyan-500)]"
                >
                  Select MIDI File
                </button>
              )}

              {preview && (
                <>
                  <button
                    onClick={() => {
                      setPreview(null);
                      setError(null);
                    }}
                    className="px-4 py-2 bg-lineHighlight text-foreground rounded hover:bg-lineBackground"
                  >
                    Choose Different File
                  </button>
                  <button
                    onClick={handleInsert}
                    className="px-4 py-2 bg-[var(--cyan-400)] text-background rounded hover:bg-[var(--cyan-500)]"
                  >
                    Insert into Editor
                  </button>
                </>
              )}

              <button
                onClick={handleCancel}
                className="px-4 py-2 bg-lineHighlight text-foreground rounded hover:bg-lineBackground"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
