# midi-to-strudel

Convert MIDI files to Strudel patterns.

## Overview

`midi-to-strudel` converts MIDI files into Strudel mini notation, making it easy to use MIDI compositions in the Strudel live coding environment. It handles both drum tracks (Channel 10) and melodic tracks, with configurable quantization and formatting options.

## Features

- **Automatic Track Detection**: Identifies drum vs. melodic tracks
- **GM Drum Mapping**: Maps General MIDI drum sounds to Strudel samples
- **Melodic Note Conversion**: Converts MIDI notes to scientific pitch notation (c4, d#5, etc.)
- **Flexible Quantization**: Configurable resolution (4, 8, 16, 32, 64 notes per bar)
- **Tempo Scaling**: Adjust playback speed
- **Pattern Compression**: Optional compaction using replication operators
- **Bar Limiting**: Extract specific sections of MIDI files

## Installation

### As a CLI Tool

```bash
cargo install midi-to-strudel
```

### As a Library

```toml
[dependencies]
midi-to-strudel = "0.1.0"
```

## CLI Usage

### Basic Conversion

```bash
# Convert a MIDI file (finds first .mid file in current directory)
midi-to-strudel

# Specify input file
midi-to-strudel --midi song.mid

# Specify output file
midi-to-strudel --midi song.mid --output song.strudel

# Print to stdout instead of file
midi-to-strudel --midi song.mid --stdout
```

### Advanced Options

```bash
# Limit to first 8 bars
midi-to-strudel --midi song.mid --bar-limit 8

# Use higher resolution (more detailed timing)
midi-to-strudel --midi song.mid --notes-per-bar 128

# Disable complex timing/chords (flat sequences only)
midi-to-strudel --midi song.mid --flat-sequences

# Adjust tempo (half speed)
midi-to-strudel --midi song.mid --tempo-scale 0.5

# Enable pattern compression
midi-to-strudel --midi song.mid --compact

# Adjust indentation
midi-to-strudel --midi song.mid --tab-size 4

# Quiet mode (suppress info messages)
midi-to-strudel --midi song.mid --quiet
```

## Output Format

The converter generates Strudel code with:

- BPM/tempo setting via `setcpm()`
- One track per MIDI track
- Comments with track names
- Mini notation patterns in backticks
- Optional `.sound()` and `.gain()` modifiers

### Example Output

```javascript
setcpm(120/4)

// Track 1: Drums
$: note(`
    [bd - sd - bd - sd -] [bd - sd - bd - sd -]
    [bd - sd - bd - sd -] [bd - sd - bd - sd -]
`).sound()

// Track 2: Piano
$: note(`
    [c4 e4 g4 -] [d4 f#4 a4 -]
    [e4 g#4 b4 -] [c4 e4 g4 -]
`).sound("piano")
```

## Library Usage

```rust
use midi_to_strudel::{MidiData, TrackBuilder, OutputFormatter};
use std::path::Path;

fn convert_midi(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    // Parse MIDI file
    let midi_data = MidiData::from_file(path)?;

    // Build tracks
    let track_builder = TrackBuilder::new(
        midi_data.cycle_len,
        0,           // bar_limit (0 = no limit)
        false,       // flat_sequences
        64,          // notes_per_bar
    );
    let tracks = track_builder.build_tracks(&midi_data.track_info);

    // Format output
    let formatter = OutputFormatter::new(2, false); // tab_size=2, compact=false
    let output = formatter.build_output(&tracks, midi_data.bpm);

    Ok(output)
}
```

## MIDI Mapping

### Drum Mapping (Channel 10)

General MIDI drums are mapped to Strudel samples:

| MIDI Note | GM Name | Strudel Sample |
|-----------|---------|----------------|
| 35-36 | Bass Drum | `bd` |
| 37-40 | Snare | `sd` |
| 41-48 | Toms | `tom` |
| 42-46 | Hi-Hat | `hh` |
| 49-52 | Crash/Ride | `cr` |
| 39 | Clap | `cp` |
| 54-81 | Percussion | `perc` |

Unknown drum notes fall back to `perc:{note_number}` format.

### Melodic Note Mapping

MIDI note numbers are converted to scientific pitch notation:

| MIDI Note | Pitch |
|-----------|-------|
| 60 | c4 |
| 61 | c#4 |
| 62 | d4 |
| 72 | c5 |

### Instrument Mapping

MIDI program changes are mapped to Strudel sounds:

| MIDI Programs | Strudel Sound |
|---------------|---------------|
| 0-7 (Piano) | `piano` |
| 8-15 (Chromatic) | `glockenspiel` |
| 24-31 (Guitar) | `gtr` |
| 32-39 (Bass) | `bass` |
| 40-47 (Strings) | `string` |
| 56-63 (Brass) | `brass` |
| 64-71 (Reed) | `reed` |
| 80-87 (Lead) | `square` |
| 88-95 (Pad) | `pad` |

## Quantization

The `--notes-per-bar` option controls timing resolution:

- **4**: Quarter note resolution (coarse)
- **8**: Eighth note resolution
- **16**: Sixteenth note resolution (default when omitted)
- **32**: 32nd note resolution
- **64**: 64th note resolution (fine, default in code)
- **128**: Very fine resolution (can produce large files)

Higher values capture more timing detail but create longer patterns.

## Pattern Compression

With `--compact`, repetitive patterns use replication:

**Without compression:**
```
bd bd bd bd sd sd sd sd
```

**With compression:**
```
bd!4 sd!4
```

## Tips

### For Best Results

1. **Clean MIDI files**: Remove unnecessary tracks (metronome, markers)
2. **Appropriate resolution**: Use 16-32 for most music, 64 for complex rhythms
3. **Bar limiting**: Extract specific sections with `--bar-limit`
4. **Tempo adjustment**: Experiment with `--tempo-scale` for different feels
5. **Flatten for simplicity**: Use `--flat-sequences` if nested patterns are too complex

### Common Workflows

**Quick preview:**
```bash
midi-to-strudel --midi song.mid --bar-limit 4 --stdout
```

**High-quality conversion:**
```bash
midi-to-strudel --midi song.mid --notes-per-bar 64 --compact
```

**Drums only:**
Open the output .strudel file and comment out non-drum tracks.

## Limitations

- Only supports Type 1 MIDI files (multiple tracks)
- Doesn't capture all MIDI nuances (pitch bend, CC, velocity curves)
- Polyphonic tracks are represented as chords in brackets
- Very fast/complex passages may need manual editing

## Integration with Strudel

Once converted, you can:

1. Copy the output into [Strudel REPL](https://strudel.cc/)
2. Edit patterns using mini notation
3. Apply Strudel effects and transformations
4. Live code on top of the converted patterns

## Examples

### Convert Drum Loop

```bash
midi-to-strudel --midi drumloop.mid --notes-per-bar 16 --compact
```

### Extract Piano Part at Half Speed

```bash
midi-to-strudel --midi song.mid --tempo-scale 0.5 --output piano.strudel
```

### Quick 8-bar Preview

```bash
midi-to-strudel --midi track.mid --bar-limit 8 --stdout | head -20
```

## License

AGPL-3.0-or-later

## Links

- [Repository](https://github.com/Emanuel-de-Jong/MIDI-To-Strudel)
- [Strudel Official Site](https://strudel.cc/)
