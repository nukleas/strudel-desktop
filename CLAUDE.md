# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Strudel is a live coding pattern language for making music in the browser. It's a port of TidalCycles to JavaScript that runs in the browser using the Web Audio API. The project uses a monorepo architecture with multiple packages managed via pnpm workspaces.

## Key Commands

### Development
```bash
pnpm i               # Install dependencies (required after cloning)
pnpm dev             # Start the REPL development server (alias: pnpm start or pnpm repl)
pnpm build           # Build the website for production
pnpm preview         # Preview the built website
```

### Testing & Quality
```bash
pnpm test            # Run all tests
pnpm test-ui         # Run tests with UI
pnpm test-coverage   # Run tests with coverage report
pnpm bench           # Run benchmarks
pnpm snapshot        # Regenerate test snapshots (use when updating/creating pattern functions)
pnpm check           # Run all CI checks (format, lint, test) - same as what PR checks run
```

### Code Quality
```bash
pnpm lint            # Check for linting errors
pnpm codeformat      # Format all files with Prettier
pnpm format-check    # Verify all files are formatted
```

### Documentation
```bash
pnpm jsdoc           # Generate documentation
pnpm jsdoc-json      # Generate JSON documentation (runs before test/build/start)
pnpm report-undocumented  # Generate report of undocumented functions
```

### Other Commands
```bash
pnpm osc             # Start OSC server (for SuperDirt/SuperCollider integration)
pnpm tauri:dev       # Start desktop app in development mode (alias: pnpm desktop)
pnpm tauri:build     # Build desktop app for production
pnpm tauri:info      # Show Tauri environment info
```

## Desktop App Features

### Queue Mode

The desktop app includes a Queue Mode feature for progressive pattern building via the AI chat assistant. Key points:

- **Location**: `docs/features/queue-system.md` and `docs/features/WAIT_CYCLES_CLARIFICATION.md`
- **Tool**: `apply_live_code_edit` in `src-tauri/src/tools.rs`
- **Timing Behavior**: `wait_cycles` is **RELATIVE/CUMULATIVE** - each change waits N cycles AFTER the previous change was applied, NOT absolute from session start
- **Implementation**: Frontend state in `website/src/repl/useReplContext.jsx`, Rust backend in `src-tauri/src/`

**Example Timeline:**
```javascript
apply_live_code_edit({ wait_cycles: 0, ... })  // Cycle 0
apply_live_code_edit({ wait_cycles: 8, ... })  // Cycle 8 (8 after first)
apply_live_code_edit({ wait_cycles: 16, ... }) // Cycle 24 (16 after second)
```

See `docs/features/WAIT_CYCLES_CLARIFICATION.md` for detailed explanation.

## Architecture

### Core Concepts

**Pattern**: The fundamental abstraction in Strudel (packages/core/pattern.mjs). A Pattern is a function that queries time spans (arcs) and returns events (Haps) that occur in that timespan. Everything in Strudel is a pattern - notes, effects, transformations, etc.

**Hap**: An event with a value and timespan, representing when and what should happen.

**TimeSpan**: Represents a span of time with begin/end as Fractions.

**Fraction**: Rational number representation for precise timing (avoids floating point errors).

### Package Structure

The codebase is organized into independent packages under `/packages`, each published to npm as `@strudel/*`:

**Core packages:**
- `core` - Core pattern engine (Pattern, Hap, TimeSpan, Fraction, evaluate, repl)
- `mini` - Mini notation parser (uses PEG.js grammar in krill.pegjs)
- `transpiler` - JavaScript code transpiler for the REPL
- `webaudio` - Web Audio API bindings (thin wrapper around superdough)
- `superdough` - Audio synthesis and sampling engine using Web Audio API

**Music theory:**
- `tonal` - Music theory utilities (scales, chords, note names)
- `xen` - Xenharmonic/microtonal utilities

**I/O packages:**
- `osc` - OSC output (for SuperDirt/SuperCollider)
- `midi` - MIDI input/output
- `serial` - Serial port communication
- `csound` - Csound integration
- `gamepad` - Gamepad input

**Visual/UI:**
- `codemirror` - CodeMirror 6 integration for the REPL editor
- `draw` - Canvas drawing utilities
- `hydra` - Hydra visual synthesis integration
- `repl` - REPL utilities

**Other:**
- `soundfonts` - SoundFont loading/playback
- `desktopbridge` - Bridge for Tauri desktop app features
- `mondo` - Alternative REPL with different evaluation model

### Evaluation Flow

1. User writes code in REPL (mini notation and/or JavaScript)
2. `transpiler` transforms code into executable JavaScript
3. `evaluate` safely evaluates the code with `evalScope` providing global functions
4. Returns a Pattern object
5. Pattern is scheduled via `repl.mjs` using NeoCyclist/Cyclist schedulers
6. Pattern queries events from time arcs
7. Events sent to output (webaudio/OSC/MIDI/etc.)
8. `superdough` renders audio via Web Audio API

### Important Files

- `packages/core/pattern.mjs` - Pattern class with all pattern methods
- `packages/core/repl.mjs` - REPL scheduler and state management
- `packages/core/evaluate.mjs` - Safe evaluation with `evalScope` for global scope
- `packages/mini/krill.pegjs` - Mini notation grammar (generates krill-parser.js)
- `packages/superdough/superdough.mjs` - Main audio engine
- `website/src/repl/` - REPL UI components (React/Astro)
- `website/src/` - Main website (Astro-based)

### Mini Notation Parser

The mini notation parser is generated from `packages/mini/krill.pegjs` using PEG.js. To regenerate after editing the grammar:
```bash
cd packages/mini
npm run build:parser
```

### String Parser

Strudel can automatically parse strings as mini notation via `setStringParser()`. When set, the `reify` function will parse all strings with it (intended for mini notation).

### Version Compatibility System

Patterns can be tagged with `// @version x.y` to preserve compatibility when breaking changes are introduced. The version tag de-activates breaking changes that came after the specified version (see CONTRIBUTING.md for database patching procedure).

## Development Workflow

### Running Single Tests

Use vitest's filter syntax:
```bash
pnpm test pattern.test.mjs
pnpm test -t "specific test name"
```

### Adding New Pattern Functions

1. Add function to appropriate package (usually `core` or a specialized package)
2. Export from package's `index.mjs`
3. Add JSDoc comments for documentation
4. Write tests in package's `test/` folder
5. Run `pnpm snapshot` to generate test snapshots if using snapshot testing
6. Function will be auto-documented via jsdoc-json

### Working with Multiple Packages

The monorepo uses pnpm workspaces. Running `pnpm i` at root symlinks all packages, allowing you to:
- Import `@strudel/<package-name>` to get local versions
- Develop multiple packages simultaneously
- Changes in one package are immediately available in others

### Publishing Packages

Always use `pnpm publish`, not `npm publish` (npm doesn't support overriding main files in publishConfig).

For bulk publishing:
```bash
npm login
npx lerna version --no-private  # Update versions
pnpm --filter "./packages/**" publish --dry-run  # Test
pnpm --filter "./packages/**" publish --access public  # Publish
```

## Technology Stack

- **Monorepo**: pnpm workspaces + lerna
- **Website**: Astro (static site generator) + React
- **Desktop**: Tauri (Rust + Web)
- **Testing**: Vitest
- **Linting**: ESLint
- **Formatting**: Prettier
- **Parser**: PEG.js (for mini notation)
- **Audio**: Web Audio API via superdough
- **Editor**: CodeMirror 6

## Important Notes

- Node.js >= 18 required
- Use pnpm (not npm) for all operations
- Never use git commands with `-i` flag (interactive not supported)
- Tests automatically run `jsdoc-json` before executing
- ESLint ignores: krill-parser.js, krill.pegjs, server.js, tidal-sniffer.js, JSX files
- Repository moved from GitHub to Codeberg (git remote: git@codeberg.org:uzu/strudel.git)
- License: AGPL-3.0-or-later

## Community & Documentation

- Docs: https://strudel.cc/learn
- Technical Manual: https://codeberg.org/uzu/strudel/wiki/Technical-Manual
- Discord: #strudel channel on TidalCycles Discord
- Forum: https://club.tidalcycles.org/
