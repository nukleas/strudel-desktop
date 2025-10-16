# Window Management

## Overview

Improve window management with proper labels, lifecycle handling, and configuration.

## Current Issues

1. **No window label**: Window accessed via `.get_webview_window("main")` but not configured with label
2. **Basic configuration**: Missing many useful window properties
3. **No window lifecycle management**: No handlers for window events
4. **Single window only**: No multi-window support structure

## Proposed Improvements

### 1. Add Window Label to Configuration

**Update `tauri.conf.json`:**

```json
{
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "Strudel REPL",
        "width": 1800,
        "height": 1200,
        "minWidth": 800,
        "minHeight": 600,
        "resizable": true,
        "maximizable": true,
        "minimizable": true,
        "closable": true,
        "fullscreen": false,
        "visible": true,
        "transparent": false,
        "decorations": true,
        "alwaysOnTop": false,
        "center": true,
        "theme": "Auto"
      }
    ]
  }
}
```

### 2. Window State Management

Create a window state manager to persist window position and size.

**Create `src/window_state.rs`:**

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub maximized: bool,
}

impl WindowState {
    pub fn save_path(app: &AppHandle) -> PathBuf {
        let mut path = app
            .path()
            .app_data_dir()
            .expect("Failed to get app data dir");
        path.push("window-state.json");
        path
    }

    pub fn load(app: &AppHandle) -> Option<Self> {
        let path = Self::save_path(app);
        let contents = fs::read_to_string(path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    pub fn save(&self, app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::save_path(app);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }

    pub fn from_window(window: &tauri::WebviewWindow) -> Result<Self, Box<dyn std::error::Error>> {
        let size = window.outer_size()?;
        let position = window.outer_position()?;
        let maximized = window.is_maximized()?;

        Ok(Self {
            width: size.width,
            height: size.height,
            x: position.x,
            y: position.y,
            maximized,
        })
    }

    pub fn apply_to_window(&self, window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
        window.set_size(PhysicalSize::new(self.width, self.height))?;
        window.set_position(PhysicalPosition::new(self.x, self.y))?;

        if self.maximized {
            window.maximize()?;
        }

        Ok(())
    }
}
```

### 3. Window Event Handlers

**Update `src/main.rs` to handle window events:**

```rust
use tauri::{Manager, WindowEvent};

fn main() {
    // ... initialization ...

    builder
        .setup(|app| {
            let window = app.get_webview_window("main")
                .ok_or("Failed to get main window")?;

            // Restore window state
            if let Some(state) = WindowState::load(app.handle()) {
                if let Err(e) = state.apply_to_window(&window) {
                    eprintln!("Failed to restore window state: {}", e);
                }
            }

            // Setup window event handlers
            let app_handle = app.handle().clone();
            window.on_window_event(move |event| {
                match event {
                    WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                        // Save window state on resize/move
                        if let Some(window) = app_handle.get_webview_window("main") {
                            if let Ok(state) = WindowState::from_window(&window) {
                                let _ = state.save(&app_handle);
                            }
                        }
                    }
                    WindowEvent::CloseRequested { .. } => {
                        // Save state before closing
                        if let Some(window) = app_handle.get_webview_window("main") {
                            if let Ok(state) = WindowState::from_window(&window) {
                                let _ = state.save(&app_handle);
                            }
                        }
                    }
                    WindowEvent::Focused(focused) => {
                        println!("Window focus changed: {}", focused);
                    }
                    _ => {}
                }
            });

            // ... rest of setup ...
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 4. Window Commands

Add commands for window manipulation:

**Create `src/window_commands.rs`:**

```rust
use tauri::{AppHandle, Manager, Window};

#[tauri::command]
pub async fn minimize_window(window: Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn maximize_window(window: Window) -> Result<(), String> {
    window.maximize().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn unmaximize_window(window: Window) -> Result<(), String> {
    window.unmaximize().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn close_window(window: Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_fullscreen(window: Window) -> Result<(), String> {
    let is_fullscreen = window.is_fullscreen().map_err(|e| e.to_string())?;
    window
        .set_fullscreen(!is_fullscreen)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_always_on_top(window: Window, always_on_top: bool) -> Result<(), String> {
    window
        .set_always_on_top(always_on_top)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_window_state(window: Window) -> Result<WindowInfo, String> {
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let position = window.outer_position().map_err(|e| e.to_string())?;
    let is_maximized = window.is_maximized().map_err(|e| e.to_string())?;
    let is_fullscreen = window.is_fullscreen().map_err(|e| e.to_string())?;
    let is_focused = window.is_focused().map_err(|e| e.to_string())?;

    Ok(WindowInfo {
        width: size.width,
        height: size.height,
        x: position.x,
        y: position.y,
        is_maximized,
        is_fullscreen,
        is_focused,
    })
}

#[derive(serde::Serialize)]
pub struct WindowInfo {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub is_maximized: bool,
    pub is_fullscreen: bool,
    pub is_focused: bool,
}
```

**Register commands in `main.rs`:**

```rust
.invoke_handler(tauri::generate_handler![
    midibridge::sendmidi,
    oscbridge::sendosc,
    window_commands::minimize_window,
    window_commands::maximize_window,
    window_commands::unmaximize_window,
    window_commands::close_window,
    window_commands::toggle_fullscreen,
    window_commands::set_always_on_top,
    window_commands::get_window_state,
])
```

### 5. Multi-Window Support (Optional)

Prepare structure for potential multi-window features:

**Create `src/windows.rs`:**

```rust
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub enum WindowLabel {
    Main,
    Settings,
    Help,
}

impl WindowLabel {
    pub fn as_str(&self) -> &str {
        match self {
            WindowLabel::Main => "main",
            WindowLabel::Settings => "settings",
            WindowLabel::Help => "help",
        }
    }
}

pub fn create_settings_window(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    if app.get_webview_window(WindowLabel::Settings.as_str()).is_some() {
        // Window already exists, just focus it
        if let Some(window) = app.get_webview_window(WindowLabel::Settings.as_str()) {
            window.set_focus()?;
        }
        return Ok(());
    }

    WebviewWindowBuilder::new(
        app,
        WindowLabel::Settings.as_str(),
        WebviewUrl::App("settings.html".into()),
    )
    .title("Strudel Settings")
    .inner_size(600.0, 400.0)
    .resizable(true)
    .center()
    .build()?;

    Ok(())
}

pub fn create_help_window(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    if app.get_webview_window(WindowLabel::Help.as_str()).is_some() {
        if let Some(window) = app.get_webview_window(WindowLabel::Help.as_str()) {
            window.set_focus()?;
        }
        return Ok(());
    }

    WebviewWindowBuilder::new(
        app,
        WindowLabel::Help.as_str(),
        WebviewUrl::App("help.html".into()),
    )
    .title("Strudel Help")
    .inner_size(800.0, 600.0)
    .resizable(true)
    .center()
    .build()?;

    Ok(())
}

#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<(), String> {
    create_settings_window(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_help(app: AppHandle) -> Result<(), String> {
    create_help_window(&app).map_err(|e| e.to_string())
}
```

### 6. Window Theming

Support system theme detection:

```rust
use tauri::{Manager, Theme};

#[tauri::command]
pub async fn get_theme(app: AppHandle) -> Result<String, String> {
    if let Some(window) = app.get_webview_window("main") {
        let theme = window.theme().map_err(|e| e.to_string())?;
        Ok(match theme {
            Theme::Light => "light".to_string(),
            Theme::Dark => "dark".to_string(),
            _ => "auto".to_string(),
        })
    } else {
        Err("Window not found".to_string())
    }
}

#[tauri::command]
pub async fn set_theme(window: Window, theme: String) -> Result<(), String> {
    let tauri_theme = match theme.as_str() {
        "light" => Some(Theme::Light),
        "dark" => Some(Theme::Dark),
        _ => None,
    };

    window.set_theme(tauri_theme).map_err(|e| e.to_string())
}
```

## Frontend Integration

**TypeScript example:**

```typescript
import { invoke } from '@tauri-apps/api/core';

// Window controls
export async function minimizeWindow() {
  await invoke('minimize_window');
}

export async function maximizeWindow() {
  await invoke('maximize_window');
}

export async function closeWindow() {
  await invoke('close_window');
}

export async function toggleFullscreen() {
  await invoke('toggle_fullscreen');
}

// Get window state
export async function getWindowState() {
  return await invoke('get_window_state');
}

// Theme
export async function getTheme() {
  return await invoke('get_theme');
}

export async function setTheme(theme: 'light' | 'dark' | 'auto') {
  await invoke('set_theme', { theme });
}
```

## Benefits

1. **Better UX**: Window state persists between sessions
2. **Native Feel**: Proper window controls and behaviors
3. **Flexibility**: Easy to add more windows
4. **Theme Support**: Respects system theme preferences
5. **Professional**: Standard desktop app behavior

## Implementation Checklist

- [ ] Add window label to configuration
- [ ] Implement window state persistence
- [ ] Add window event handlers
- [ ] Create window commands
- [ ] Add theme support
- [ ] Test window state saving/loading
- [ ] Test window controls from frontend
- [ ] Verify multi-monitor support
- [ ] Test on all platforms (Windows, macOS, Linux)

## Testing

- [ ] Test window state persistence across restarts
- [ ] Test minimize/maximize/restore
- [ ] Test fullscreen mode
- [ ] Test window movement between monitors
- [ ] Test theme switching
- [ ] Test with different screen resolutions
- [ ] Test window size constraints (min/max)
