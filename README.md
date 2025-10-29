# 🎵 Strudel Desktop

> Live coding music patterns on your desktop - a native desktop application for [Strudel](https://strudel.cc)

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%203.0-blue.svg)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/Built%20with-Tauri-FFC131?logo=tauri)](https://tauri.app)
[![Powered by Rust](https://img.shields.io/badge/Powered%20by-Rust-orange?logo=rust)](https://www.rust-lang.org/)

**Desktop Repository**: https://github.com/nukleas/strudel-desktop
**Original Project**: https://codeberg.org/uzu/strudel

---

## 🎥 Demo Video

[![Strudel Desktop Demo](https://img.youtube.com/vi/jivheA_U6uA/maxresdefault.jpg)](https://www.youtube.com/watch?v=jivheA_U6uA)

**[Watch the Demo →](https://www.youtube.com/watch?v=jivheA_U6uA)**

---

## About

Strudel Desktop brings the power of [Strudel](https://strudel.cc) - the live coding pattern language - to your desktop. Write and perform music using JavaScript and mini notation, with full access to your local file system, MIDI devices, and desktop integrations.

Strudel is a JavaScript port of [TidalCycles](https://tidalcycles.org/), running entirely in the browser via Web Audio API, now wrapped in a native desktop shell using Tauri.

## ✨ Features

### 🎹 Core Music Creation
- **Live Coding REPL** - Write and evaluate patterns in real-time
- **Mini Notation** - Compact pattern syntax inspired by TidalCycles
- **Web Audio Engine** - Built-in synthesis, sampling, and effects
- **Sample Library** - Load and use samples from your local file system
- **Pattern Visualization** - Real-time piano roll and pattern display

### 🖥️ Desktop Features
- **Native Performance** - Rust/Tauri backend for optimal performance
- **File System Access** - Save/load patterns locally
- **MIDI Support** - Connect MIDI controllers and devices
- **OSC Integration** - Connect to SuperCollider and other audio software
- **Cross-Platform** - Works on macOS, Windows, and Linux

### 🤖 AI-Powered Assistant
- **Chat Interface** - Ask questions and get help writing patterns
- **Multi-Provider Support** - Works with Claude (Anthropic), GPT (OpenAI), or Gemini (Google)
- **Context-Aware** - Understands Strudel syntax and musical concepts
- **Code Generation** - Generate patterns from natural language
- **Progressive Building** - Queue system for step-by-step pattern construction
- **Documentation Search** - Semantic search over Strudel documentation (RAG)

### 🎬 Queue System
- **Progressive Patterns** - Build complexity gradually with timed changes
- **Auto-Advance** - Changes apply automatically with musical pacing
- **Manual Control** - Preview, skip, or apply changes on demand
- **Live Coding Friendly** - Perfect for performances and teaching

### 🔍 Smart Documentation
- **Offline RAG Search** - Semantic search without internet connection
- **Function Lookup** - Find the right Strudel function quickly
- **Example Library** - Hundreds of example patterns built-in

---

## 📦 Installation

### Option 1: Download Pre-Built Binaries

Download the latest build for your platform from [GitHub Actions](https://github.com/nukleas/strudel-desktop/actions) or [Releases](https://github.com/nukleas/strudel-desktop/releases):

- **macOS**: `.dmg` or `.app` - See [INSTALL_MACOS.md](INSTALL_MACOS.md) for fixing "damaged app" errors
- **Linux**: `.deb` or `.AppImage`
- **Windows**: `.exe` or `.msi`

### Option 2: Build from Source

#### Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [pnpm](https://pnpm.io/) (recommended) or npm
- [Rust](https://rustup.rs/)

#### Quick Start

1. **Clone the repository**
   ```bash
   git clone https://github.com/nukleas/strudel-desktop.git
   cd strudel-desktop
   ```

2. **Install dependencies**
   ```bash
   pnpm install
   ```

3. **Run in development mode**
   ```bash
   pnpm desktop
   # or: pnpm tauri:dev
   ```

4. **Build for production**
   ```bash
   pnpm tauri:build
   ```

   Built app will be in `src-tauri/target/release/bundle/`

### Building for Specific Platforms

The project supports cross-platform builds for Windows, macOS, and Linux:

**macOS (Apple Silicon):**
```bash
pnpm tauri:build -- --target aarch64-apple-darwin
```

**macOS (Intel):**
```bash
pnpm tauri:build -- --target x86_64-apple-darwin
```

**Linux (x86_64):**
```bash
pnpm tauri:build -- --target universal-linux-gnu
```

**Windows (x86_64):**
```bash
pnpm tauri:build
```

**Output Locations:**
- **macOS**: `src-tauri/target/[arch]/release/bundle/dmg/` and `.app`
- **Linux**: `src-tauri/target/release/bundle/deb/` and `.AppImage`
- **Windows**: `src-tauri/target/release/bundle/nsis/` (`.exe`) and `.msi`

### Automated Builds

GitHub Actions automatically builds for all platforms on every push to `main` or `develop` branches. Download pre-built binaries from the [Actions](https://github.com/nukleas/strudel-desktop/actions) tab or [Releases](https://github.com/nukleas/strudel-desktop/releases) page.

---

## 🚀 Usage

### Basic Patterns

```javascript
// Simple drum pattern
s("bd hh sd hh")

// Melodic pattern with scale
n("0 2 4 7").scale("C:minor").s("sine")

// Effects chain
s("bd sd").delay(0.5).room(0.8)

// Layering patterns
stack(
  s("bd*4"),
  s("~ sd ~ sd"),
  n("0 4 7 12").scale("C4:minor").s("triangle")
)
```

### AI Chat Assistant

1. **Set up API key** - See [Chat Assistant Setup](docs/features/chat-assistant.md)
2. **Enable Chat tab** - Click the chat icon in the panel
3. **Ask questions**:
   ```
   "Create a techno beat"
   "How do I add reverb?"
   "Explain the euclidean rhythm function"
   ```
4. **Use Queue Mode** (🎬) for progressive pattern building

### Loading Local Samples

```javascript
// Load from your music directory
samples('/path/to/my/samples')

// Use in patterns
s("kick snare:2 kick snare:3")
```

---

## 📖 Documentation

- **[Features Documentation](docs/)** - Detailed feature guides
  - [Chat Assistant Setup](docs/features/chat-assistant.md)
  - [Queue System](docs/features/queue-system.md)
  - [RAG Search](docs/features/rag-search.md)
- **[Strudel Learn](https://strudel.cc/learn)** - Official Strudel tutorials
- **[Technical Manual](https://codeberg.org/uzu/strudel/wiki/Technical-Manual)** - Architecture and internals
- **[Contributing](CONTRIBUTING.md)** - Development guidelines

---

## 🛠️ Development

### Available Commands

| Command | Description |
|---------|-------------|
| `pnpm desktop` | Start desktop app in development mode |
| `pnpm tauri:dev` | Alternative dev mode command |
| `pnpm tauri:build` | Build for production |
| `pnpm dev` | Start web development server |
| `pnpm test` | Run test suite |
| `pnpm test-ui` | Run tests with UI |
| `pnpm lint` | Check code style |
| `pnpm codeformat` | Format code with Prettier |

### Project Structure

```
strudel-desktop/
├── src-tauri/           # Rust/Tauri backend
│   ├── src/
│   │   ├── chatbridge.rs    # AI chat integration
│   │   ├── tools.rs         # AI tool definitions
│   │   ├── rag/             # RAG search system
│   │   └── ...
│   └── Cargo.toml
├── packages/            # JavaScript packages (monorepo)
│   ├── core/           # Pattern engine
│   ├── mini/           # Mini notation parser
│   ├── webaudio/       # Audio synthesis
│   └── ...
├── website/            # Web UI (used by desktop)
│   └── src/repl/       # REPL components
└── docs/               # Documentation
```

### Building from Source

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed build instructions and development setup.

---

## 🤝 Contributing

Contributions are welcome! Whether it's bug reports, feature requests, or code contributions.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

---

## 📜 License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0-or-later)**.

See [LICENSE](LICENSE) for the full license text.

### What This Means

- ✅ You can use, modify, and distribute this software
- ✅ You must share modifications under the same license
- ✅ You must disclose source code when distributing
- ✅ Network use counts as distribution (AGPL clause)

---

## 🙏 Acknowledgments

- **[Strudel](https://codeberg.org/uzu/strudel)** - The original web-based live coding environment
- **[TidalCycles](https://tidalcycles.org/)** - The inspiration for pattern-based music coding
- **[Tauri](https://tauri.app/)** - The desktop framework
- **[Anthropic](https://anthropic.com/)** - Claude API for chat features
- The live coding and algorave community

---

## 🔗 Links

- **Desktop App**: [github.com/nukleas/strudel-desktop](https://github.com/nukleas/strudel-desktop)
- **Original Strudel**: [codeberg.org/uzu/strudel](https://codeberg.org/uzu/strudel)
- **Strudel Website**: [strudel.cc](https://strudel.cc)
- **TidalCycles**: [tidalcycles.org](https://tidalcycles.org/)
- **Discord**: [TidalCycles Discord](https://discord.gg/tidal) (#strudel channel)
- **Forum**: [club.tidalcycles.org](https://club.tidalcycles.org/)

---

<p align="center">
  <strong>Made with ❤️ by the live coding community</strong>
</p>
