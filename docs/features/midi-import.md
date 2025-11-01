# MIDI Import Feature

The Strudel desktop app can import MIDI files and convert them to Strudel patterns using the Rust-powered MIDI parser.

## Usage from Frontend

### TypeScript API

```typescript
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

// Show file picker for MIDI files
const filePath = await open({
  title: 'Import MIDI File',
  filters: [
    {
      name: 'MIDI Files',
      extensions: ['mid', 'midi']
    }
  ]
});

if (filePath) {
  try {
    // Convert MIDI to Strudel code
    const strudelCode = await invoke('import_midi_file', {
      filePath,
      options: {
        barLimit: 0,         // 0 = unlimited
        compact: true,       // Use ! operator for repetitions
        tempoScale: 1.0,     // 1.0 = original tempo
        notesPerBar: 64,     // Quantization resolution
        tabSize: 2           // Indentation
      }
    });

    // Insert into editor or display
    console.log(strudelCode);
  } catch (error) {
    console.error('MIDI import failed:', error);
  }
}
```

### Conversion Options

```typescript
interface MidiConversionOptions {
  /// Maximum number of bars to convert (0 = unlimited)
  barLimit?: number;

  /// Use compact notation (compress repetitions with ! operator)
  compact?: boolean;

  /// Tempo scaling factor (1.0 = original tempo, 2.0 = double speed)
  tempoScale?: number;

  /// Number of notes per bar for quantization (higher = more precise)
  notesPerBar?: number;

  /// Indentation size in spaces
  tabSize?: number;
}
```

## Example Integration

### React Component

```tsx
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export function MidiImportButton() {
  const [importing, setImporting] = useState(false);

  const handleImport = async () => {
    setImporting(true);
    try {
      const filePath = await open({
        title: 'Import MIDI File',
        filters: [
          {
            name: 'MIDI Files',
            extensions: ['mid', 'midi']
          }
        ]
      });

      if (!filePath) {
        return; // User cancelled
      }

      const strudelCode = await invoke('import_midi_file', {
        filePath,
        options: {
          compact: true,
          tempoScale: 1.0
        }
      });

      // Insert into CodeMirror editor
      editor.dispatch({
        changes: {
          from: editor.state.selection.main.head,
          insert: strudelCode
        }
      });

    } catch (error) {
      console.error('Failed to import MIDI:', error);
      alert(`Import failed: ${error}`);
    } finally {
      setImporting(false);
    }
  };

  return (
    <button onClick={handleImport} disabled={importing}>
      {importing ? 'Importing...' : 'Import MIDI'}
    </button>
  );
}
```

### With Preview Dialog

```tsx
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export function MidiImportDialog() {
  const [preview, setPreview] = useState<string | null>(null);
  const [options, setOptions] = useState({
    compact: true,
    tempoScale: 1.0,
    notesPerBar: 64
  });

  const handleSelectFile = async () => {
    const filePath = await open({
      title: 'Import MIDI File',
      filters: [{ name: 'MIDI Files', extensions: ['mid', 'midi'] }]
    });

    if (filePath) {
      const code = await invoke('import_midi_file', {
        filePath,
        options
      });
      setPreview(code);
    }
  };

  const handleInsert = () => {
    if (preview) {
      // Insert into editor
      editor.setValue(preview);
      setPreview(null);
    }
  };

  return (
    <div>
      <div>
        <label>
          <input
            type="checkbox"
            checked={options.compact}
            onChange={e => setOptions({...options, compact: e.target.checked})}
          />
          Compact mode (use ! for repetitions)
        </label>
      </div>

      <div>
        <label>
          Tempo scale:
          <input
            type="number"
            step="0.1"
            value={options.tempoScale}
            onChange={e => setOptions({...options, tempoScale: parseFloat(e.target.value)})}
          />
        </label>
      </div>

      <button onClick={handleSelectFile}>
        Select MIDI File
      </button>

      {preview && (
        <div>
          <h3>Preview:</h3>
          <pre>{preview}</pre>
          <button onClick={handleInsert}>Insert</button>
          <button onClick={() => setPreview(null)}>Cancel</button>
        </div>
      )}
    </div>
  );
}
```

## Output Format

The MIDI importer generates Strudel code in this format:

```javascript
setcpm(120/4)

// Track 1: Drums
$: s(`bd sd hh sd`)

// Track 2: Bass
$: note(`c2 e2 g2 c3`).sound("bass")

// Track 3: Piano
$: note(`c4 e4 g4 c5`).sound("piano")
```

### Track Types

#### Drum Tracks (MIDI Channel 10)
- Uses `s()` syntax with sample names
- Maps MIDI drum notes to Strudel samples:
  - C1 → `bd` (bass drum)
  - D1, E1 → `sd` (snare drum)
  - F#1, G#1 → `hh` (hi-hat)
  - A1 → `oh` (open hi-hat)
  - Others → `perc` (percussion)

#### Melodic Tracks
- Uses `note()` syntax with note names (e.g., `c4`, `ds3`)
- Maps General MIDI program numbers to instruments:
  - 0-7 → `piano`
  - 8-15 → `glockenspiel`
  - 16-23 → `organ`
  - 24-31 → `guitar`
  - 32-39 → `bass`
  - 40-47 → `strings`
  - And more...

## Security

The MIDI import feature uses Tauri 2's security model:

- **User must explicitly select files** via the file dialog
- **No automatic file access** - files are only read when user chooses them
- **Permissions defined** in `src-tauri/capabilities/default.json`:
  - `dialog:allow-open` - Show file picker
  - `fs:allow-read-file` - Read user-selected files

## Error Handling

```typescript
try {
  const code = await invoke('import_midi_file', {
    filePath,
    options
  });
} catch (error) {
  // Error types:
  // - File not found
  // - Invalid MIDI file
  // - Parse errors
  // - IO errors
  console.error('Import failed:', error);
}
```

## Limitations & Future Enhancements

### Current Limitations
- Simplified timing (all notes in sequence)
- Basic drum mapping (limited to common drums)
- No chord detection
- No velocity-based dynamics yet

### Planned Enhancements
- Advanced timing with proper quantization
- Bar/beat awareness
- Chord detection and representation
- Velocity → gain mapping
- Track selection (import only specific tracks)
- Visual MIDI preview before import
- Polyphonic pattern generation
- Better drum mapping based on General MIDI standard

## Advanced Usage

### Custom File Dialog

```typescript
import { open } from '@tauri-apps/plugin-dialog';

// With custom options
const filePath = await open({
  title: 'Import MIDI File',
  directory: false,
  multiple: false,
  filters: [
    {
      name: 'MIDI Files',
      extensions: ['mid', 'midi', 'MID', 'MIDI']
    },
    {
      name: 'All Files',
      extensions: ['*']
    }
  ],
  defaultPath: await homeDir()
});
```

### Batch Import

```typescript
import { open } from '@tauri-apps/plugin-dialog';

// Allow multiple file selection
const filePaths = await open({
  title: 'Import MIDI Files',
  multiple: true,
  filters: [{ name: 'MIDI Files', extensions: ['mid', 'midi'] }]
});

if (Array.isArray(filePaths)) {
  const patterns = await Promise.all(
    filePaths.map(path =>
      invoke('import_midi_file', { filePath: path })
    )
  );

  // Combine patterns
  const combined = patterns.join('\n\n');
}
```

## Menu Integration

Add to your app menu:

```typescript
// In Tauri menu configuration
{
  label: 'File',
  submenu: [
    {
      label: 'Import MIDI...',
      accelerator: 'CmdOrCtrl+I',
      click: () => {
        // Trigger MIDI import
        window.dispatchEvent(new CustomEvent('midi-import'));
      }
    }
  ]
}
```

## Troubleshooting

### "Failed to read MIDI file"
- Check file path is correct
- Ensure file exists and is readable
- Verify file has `.mid` or `.midi` extension

### "Failed to parse MIDI file"
- File may be corrupted
- File may not be valid MIDI format
- Try opening in another MIDI tool to verify

### "No tracks found"
- MIDI file has no note events
- All tracks may be empty
- Try importing a different MIDI file

## See Also

- [Rust Pattern Commands](./rust-pattern-commands.md)
- [Queue System](./queue-system.md)
- [Tauri Dialog Plugin](https://tauri.app/plugin/dialog/)
- [Tauri FS Plugin](https://tauri.app/plugin/file-system/)
