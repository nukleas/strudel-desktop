# @strudel/tauri

Rust source files for building native desktop apps using Tauri

## Usage

Install [Rust](https://rustup.rs/) on your system.

From the project root:

- install Strudel dependencies

```bash
pnpm i
```

- to run Strudel for development (desktop app)

```bash
npm run tauri:dev
# or
npm run desktop
```

- to build the binary and installer/bundle

```bash
npm run tauri:build
```

- to get Tauri environment information

```bash
npm run tauri:info
```

The binary and installer can be found in the 'src-tauri/target/release/bundle' directory

## Available Scripts

- `npm run tauri:dev` - Start the desktop app in development mode
- `npm run desktop` - Alias for `tauri:dev` (easier to remember)
- `npm run tauri:build` - Build the desktop app for production
- `npm run tauri:info` - Show Tauri environment and configuration info
- `npm run tauri` - Run any Tauri CLI command directly
