# DevTools Integration

## Overview

Add Tauri DevTools plugin for easier debugging and development. This provides WebView inspection, performance monitoring, and runtime diagnostics.

## Current State

The app currently has no DevTools integration. To inspect the WebView, developers must manually trigger DevTools via code or keyboard shortcuts.

## Proposed Solution

### 1. Add DevTools Plugin Dependency

**Update `Cargo.toml`:**

```toml
[dependencies]
# ... existing dependencies ...
tauri-plugin-devtools = "2.0"
```

### 2. Initialize DevTools in Main

**Update `src/main.rs`:**

```rust
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod loggerbridge;
mod midibridge;
mod oscbridge;

use std::sync::Arc;
use loggerbridge::Logger;
use tauri::Manager;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

#[derive(Clone, serde::Serialize)]
struct Payload {
    message: String,
    message_type: String,
}

fn main() {
    // Initialize DevTools as early as possible (debug builds only)
    #[cfg(debug_assertions)]
    let devtools = tauri_plugin_devtools::init();

    let (async_input_transmitter_midi, async_input_receiver_midi) = mpsc::channel(1);
    let (async_output_transmitter_midi, async_output_receiver_midi) = mpsc::channel(1);
    let (async_input_transmitter_osc, async_input_receiver_osc) = mpsc::channel(1);
    let (async_output_transmitter_osc, async_output_receiver_osc) = mpsc::channel(1);

    let mut builder = tauri::Builder::default()
        .manage(midibridge::AsyncInputTransmit {
            inner: Mutex::new(async_input_transmitter_midi),
        })
        .manage(oscbridge::AsyncInputTransmit {
            inner: Mutex::new(async_input_transmitter_osc),
        })
        .invoke_handler(tauri::generate_handler![
            midibridge::sendmidi,
            oscbridge::sendosc
        ])
        .plugin(tauri_plugin_clipboard_manager::init());

    // Add DevTools plugin in debug builds
    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(devtools);
    }

    builder
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // Optionally auto-open DevTools in debug builds
            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }

            let logger = Logger {
                window: Arc::new(window),
            };

            midibridge::init(
                logger.clone(),
                async_input_receiver_midi,
                async_output_receiver_midi,
                async_output_transmitter_midi,
            );

            oscbridge::init(
                logger,
                async_input_receiver_osc,
                async_output_receiver_osc,
                async_output_transmitter_osc,
            );

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 3. Add DevTools Commands (Optional)

Create commands to toggle DevTools from the frontend:

**Add to `main.rs`:**

```rust
#[tauri::command]
#[cfg(debug_assertions)]
fn toggle_devtools(window: tauri::Window) {
    if window.is_devtools_open() {
        window.close_devtools();
    } else {
        window.open_devtools();
    }
}

// Update invoke_handler:
.invoke_handler(tauri::generate_handler![
    midibridge::sendmidi,
    oscbridge::sendosc,
    #[cfg(debug_assertions)]
    toggle_devtools
])
```

**Frontend usage:**

```javascript
import { invoke } from '@tauri-apps/api/core';

// Toggle DevTools with keyboard shortcut or button
document.addEventListener('keydown', (e) => {
  if (e.ctrlKey && e.shiftKey && e.key === 'I') {
    invoke('toggle_devtools');
  }
});
```

## Alternative: Tauri v2 DevTools Plugin

For even more features, consider using the enhanced DevTools plugin:

### Add Enhanced DevTools

```toml
[dependencies]
tauri-plugin-devtools = "2.0"
tauri-plugin-devtools-app = "2.0"  # Additional features
```

```rust
fn main() {
    let mut builder = tauri::Builder::default();

    #[cfg(debug_assertions)]
    {
        builder = builder
            .plugin(tauri_plugin_devtools::init())
            .plugin(tauri_plugin_devtools_app::init());
    }

    builder.run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Benefits

1. **Easier Debugging**: Inspect DOM, console logs, network requests
2. **Performance Profiling**: Monitor WebView performance
3. **Zero Production Overhead**: Only included in debug builds
4. **Standard DevTools**: Familiar Chrome DevTools interface
5. **Hot Reload Friendly**: Works seamlessly with dev server

## Configuration Options

### Auto-Open DevTools

```rust
#[cfg(debug_assertions)]
{
    let window = app.get_webview_window("main").unwrap();
    window.open_devtools();
}
```

### Keyboard Shortcut

Add to `tauri.conf.json`:

```json
{
  "app": {
    "windows": [
      {
        "devTools": true  // Enable DevTools
      }
    ]
  }
}
```

### Programmatic Control

```rust
use tauri::Manager;

// Open DevTools
window.open_devtools();

// Close DevTools
window.close_devtools();

// Check if open
let is_open = window.is_devtools_open();
```

## Environment-Specific Behavior

The plugin is configured to only compile in debug builds using `#[cfg(debug_assertions)]`:

- **Debug builds** (`cargo tauri dev`): DevTools available
- **Release builds** (`cargo tauri build`): DevTools code removed entirely
- **Zero overhead**: No runtime cost in production

## Implementation Checklist

- [ ] Add `tauri-plugin-devtools` to Cargo.toml
- [ ] Initialize plugin in main.rs with conditional compilation
- [ ] Test DevTools opens in debug mode
- [ ] Verify DevTools code is excluded from release builds
- [ ] (Optional) Add toggle command for frontend control
- [ ] (Optional) Configure auto-open behavior
- [ ] Document DevTools keyboard shortcuts for team

## Resources

- [Tauri DevTools Plugin Docs](https://v2.tauri.app/plugin/devtools/)
- [CrabNebula DevTools](https://github.com/crabnebula-dev/devtools) - Enhanced alternative
- [WebView Debugging Guide](https://v2.tauri.app/develop/debug/)
