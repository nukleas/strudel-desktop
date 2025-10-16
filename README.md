# Strudel Desktop

A desktop application for live coding music patterns, built with Tauri.

**Repository**: https://github.com/nukleas/strudel-desktop  
**Original Project**: https://codeberg.org/uzu/strudel

## About

This is a desktop-focused fork of Strudel, the live coding pattern language for making music. Strudel brings TidalCycles to JavaScript and runs natively on desktop platforms using Tauri.

## Features

- 🎵 **Live Coding**: Write music patterns in real-time
- 🖥️ **Desktop Native**: Full desktop app experience with file system access
- 🔊 **Audio Engine**: Built-in Web Audio API for synthesis and effects
- 📁 **Local Samples**: Load samples from your local file system
- 🎛️ **MIDI Support**: Connect MIDI controllers and devices
- 🌐 **OSC Integration**: Connect to SuperCollider and other audio software

## Installation

### Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [pnpm](https://pnpm.io/)
- [Rust](https://rustup.rs/) (for Tauri)

### Development Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/nukleas/strudel-desktop.git
   cd strudel-desktop
   ```

2. Install dependencies:
   ```bash
   pnpm i
   ```

3. Run in development mode:
   ```bash
   pnpm desktop
   ```

### Building

Build the desktop app for production:

```bash
 pnpm tauri:build
```

The built app will be in `src-tauri/target/release/bundle/`

## Usage

### Basic Patterns

```javascript
// Simple drum pattern
d1("bd hh sd hh")

// Melodic pattern
d2("c3 e3 g3 b3")

// Effects
d3("c3 e3 g3 b3").saw().lpf(800)
```

### Loading Local Samples

```javascript
// Load samples from your music directory
samples('~/music/my_samples');

// Use them in patterns
d1("kick snare kick snare")
```

## Development

### Available Commands

- `pnpm desktop` - Start desktop app in development mode
- `pnpm tauri:build` - Build desktop app for production
- `pnpm dev` - Start web development server
- `pnpm test` - Run tests
- `pnpm lint` - Check code style

### Project Structure

- `src-tauri/` - Rust/Tauri backend code
- `packages/` - JavaScript packages (core, audio, UI, etc.)
- `website/` - Web frontend (used by desktop app)

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0).

See [LICENSE](LICENSE) for details.

## Contributing

This is a fork focused on desktop development. For contributing to the main Strudel project, see the [original repository](https://codeberg.org/uzu/strudel).

## Acknowledgments

- Original Strudel project by [uzu](https://codeberg.org/uzu) and contributors
- TidalCycles by Alex McLean and contributors
- Built with [Tauri](https://tauri.app/)

## Links

- [Strudel Web REPL](https://strudel.cc)
- [Documentation](https://strudel.cc/learn)
- [TidalCycles Discord](https://discord.com/invite/HGEdXmRkzT)
- [Tidal Club Forum](https://club.tidalcycles.org/)