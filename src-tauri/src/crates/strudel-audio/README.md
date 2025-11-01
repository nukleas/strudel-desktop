# strudel-audio

Native audio playback engine for Strudel patterns in Rust.

## Features

- **Self-contained**: Bundles essential drum samples for offline playback
- **HTTP fallback**: Can load additional samples from remote URLs
- **Cross-platform**: Works on macOS, Linux, and Windows via `cpal`
- **Low latency**: Real-time audio scheduling with precise timing
- **Format support**: Decodes WAV, MP3, OGG via `symphonia`

## Architecture

- **AudioEngine**: `cpal`-based audio output stream
- **SampleLoader**: Loads and caches audio samples (bundled or HTTP)
- **Scheduler**: Queries patterns and triggers samples at precise times
- **Voice**: Individual sample playback with speed/gain control
- **Player**: High-level API for playing Strudel patterns

## Usage

```rust
use strudel_audio::{Player, PlayerConfig};
use strudel_core::{pure, sequence, Value};

// Create a player
let player = Player::with_defaults()?;

// Create a pattern
let pattern = sequence(vec![
    pure(Value::String("bd".to_string())),
    pure(Value::String("sd".to_string())),
    pure(Value::String("hh".to_string())),
    pure(Value::String("hh".to_string())),
]);

// Play it
player.play(pattern)?;

// Stop when done
player.stop()?;
```

## Status

⚠️ **Work in Progress** - Core infrastructure is complete, but samples are not yet bundled.

### Completed
- ✅ Audio engine with `cpal`
- ✅ Sample loading with `symphonia`
- ✅ Pattern scheduler
- ✅ Voice playback
- ✅ High-level Player API

### TODO
- [ ] Download and bundle essential drum samples
- [ ] Implement HTTP fallback for sample loading
- [ ] Add tests with real audio samples
- [ ] Optimize for low latency
- [ ] Add pitch shifting and effects

## License

AGPL-3.0-or-later
