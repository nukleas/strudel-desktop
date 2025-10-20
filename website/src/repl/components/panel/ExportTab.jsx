import { useCallback, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';

export function ExportTab({ code }) {
  const [exporting, setExporting] = useState(false);
  const [format, setFormat] = useState('wav');
  const [duration, setDuration] = useState(10);
  const [sampleRate, setSampleRate] = useState(48000);
  const [bitDepth, setBitDepth] = useState(16);
  const [mp3Bitrate, setMp3Bitrate] = useState(320);
  const [status, setStatus] = useState('');

  const handleExport = useCallback(async () => {
    try {
      setExporting(true);
      setStatus('Opening save dialog...');

      // Show save dialog
      const outputPath = await save({
        filters: [{
          name: format.toUpperCase() + ' Audio',
          extensions: [format]
        }],
        defaultPath: `pattern.${format}`
      });

      if (!outputPath) {
        setStatus('Export cancelled');
        setExporting(false);
        return;
      }

      setStatus(`Exporting to ${format.toUpperCase()}...`);

      // Get pattern code (for now using test tone, will integrate with actual pattern later)
      const patternCode = code || 's("bd hh")';

      // Call Rust backend
      const result = await invoke('export_pattern_audio', {
        patternCode,
        params: {
          outputPath,
          format,
          sampleRate,
          channels: 2,
          durationCycles: duration,
          bitDepth: format === 'wav' ? bitDepth : null,
          mp3Bitrate: format === 'mp3' ? mp3Bitrate : null,
        }
      });

      setStatus(`✅ Export successful: ${result}`);
    } catch (error) {
      console.error('Export failed:', error);
      setStatus(`❌ Export failed: ${error}`);
    } finally {
      setExporting(false);
    }
  }, [code, format, duration, sampleRate, bitDepth, mp3Bitrate]);

  return (
    <div className="p-4">
      <h2 className="text-xl font-bold mb-4">Export Audio</h2>

      <div className="space-y-4">
        <div>
          <label className="block text-sm font-medium mb-1">
            Format
          </label>
          <select
            value={format}
            onChange={e => setFormat(e.target.value)}
            className="w-full p-2 border rounded"
            disabled={exporting}
          >
            <option value="wav">WAV</option>
            <option value="mp3">MP3 (Not yet implemented)</option>
          </select>
        </div>

        <div>
          <label className="block text-sm font-medium mb-1">
            Duration (cycles)
          </label>
          <input
            type="number"
            value={duration}
            onChange={e => setDuration(Number(e.target.value))}
            className="w-full p-2 border rounded"
            min="1"
            max="300"
            disabled={exporting}
          />
        </div>

        <div>
          <label className="block text-sm font-medium mb-1">
            Sample Rate
          </label>
          <select
            value={sampleRate}
            onChange={e => setSampleRate(Number(e.target.value))}
            className="w-full p-2 border rounded"
            disabled={exporting}
          >
            <option value={44100}>44.1 kHz</option>
            <option value={48000}>48 kHz</option>
            <option value={96000}>96 kHz</option>
          </select>
        </div>

        {format === 'wav' && (
          <div>
            <label className="block text-sm font-medium mb-1">
              Bit Depth
            </label>
            <select
              value={bitDepth}
              onChange={e => setBitDepth(Number(e.target.value))}
              className="w-full p-2 border rounded"
              disabled={exporting}
            >
              <option value={16}>16-bit</option>
              <option value={24}>24-bit</option>
              <option value={32}>32-bit (float)</option>
            </select>
          </div>
        )}

        {format === 'mp3' && (
          <div>
            <label className="block text-sm font-medium mb-1">
              Bitrate
            </label>
            <select
              value={mp3Bitrate}
              onChange={e => setMp3Bitrate(Number(e.target.value))}
              className="w-full p-2 border rounded"
              disabled={exporting}
            >
              <option value={128}>128 kbps</option>
              <option value={192}>192 kbps</option>
              <option value={256}>256 kbps</option>
              <option value={320}>320 kbps</option>
            </select>
          </div>
        )}

        <div className="p-3 bg-blue-50 border border-blue-200 rounded text-sm">
          <strong>Info:</strong> Exports your current pattern using the Dough audio engine.
          <br />
          <strong>Note:</strong> Visualization methods like <code>.pianoroll()</code> are automatically removed during export.
          <br />
          Leave the editor empty to export a test tone (440 Hz).
        </div>

        <button
          onClick={handleExport}
          disabled={exporting}
          className={`w-full p-3 rounded font-medium ${
            exporting
              ? 'bg-gray-300 cursor-not-allowed'
              : 'bg-blue-500 hover:bg-blue-600 text-white'
          }`}
        >
          {exporting ? 'Exporting...' : `Export as ${format.toUpperCase()}`}
        </button>

        {status && (
          <div className="p-3 bg-gray-100 border rounded text-sm">
            {status}
          </div>
        )}
      </div>
    </div>
  );
}
