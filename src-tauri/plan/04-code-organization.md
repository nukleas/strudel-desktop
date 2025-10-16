# Code Organization & Architecture

## Overview

Refactor the MIDI and OSC bridges into proper Tauri plugins with better separation of concerns and maintainability.

## Current Issues

1. **Bridges are not proper plugins**: They're just modules with `init()` functions
2. **Mixed concerns**: Message processing, state management, and I/O in same modules
3. **No shared abstractions**: MIDI and OSC bridges have duplicate code patterns
4. **State management**: State is managed via raw Tokio channels and mutexes
5. **No clear lifecycle**: Setup happens in `main.rs` with manual channel wiring

## Proposed Architecture

### Plugin Structure

```
src-tauri/
├── src/
│   ├── main.rs              # App entry point
│   ├── lib.rs               # Shared library (for potential mobile support)
│   ├── error.rs             # Shared error types
│   ├── logger.rs            # Renamed from loggerbridge
│   ├── plugins/
│   │   ├── mod.rs
│   │   ├── midi/
│   │   │   ├── mod.rs       # Plugin definition
│   │   │   ├── types.rs     # Message types
│   │   │   ├── commands.rs  # Tauri commands
│   │   │   ├── state.rs     # Plugin state
│   │   │   └── processor.rs # Message processing logic
│   │   └── osc/
│   │       ├── mod.rs       # Plugin definition
│   │       ├── types.rs     # Message types
│   │       ├── commands.rs  # Tauri commands
│   │       ├── state.rs     # Plugin state
│   │       └── processor.rs # Message processing logic
│   └── messaging/
│       ├── mod.rs
│       ├── queue.rs         # Shared message queue abstraction
│       └── scheduler.rs     # Shared scheduling logic
```

## Implementation

### 1. Shared Error Type

**Create `src/error.rs`:**

```rust
use std::fmt;

#[derive(Debug)]
pub enum StrudelError {
    MidiInit(String),
    MidiPortOpen(String),
    MidiSend(String),
    OscInit(String),
    OscSend(String),
    WindowNotFound(String),
    InvalidMessage(String),
}

impl fmt::Display for StrudelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StrudelError::MidiInit(msg) => write!(f, "MIDI initialization error: {}", msg),
            StrudelError::MidiPortOpen(msg) => write!(f, "MIDI port error: {}", msg),
            StrudelError::MidiSend(msg) => write!(f, "MIDI send error: {}", msg),
            StrudelError::OscInit(msg) => write!(f, "OSC initialization error: {}", msg),
            StrudelError::OscSend(msg) => write!(f, "OSC send error: {}", msg),
            StrudelError::WindowNotFound(msg) => write!(f, "Window not found: {}", msg),
            StrudelError::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
        }
    }
}

impl std::error::Error for StrudelError {}

pub type Result<T> = std::result::Result<T, StrudelError>;
```

### 2. Shared Message Queue

**Create `src/messaging/queue.rs`:**

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

pub trait TimedMessage: Send + Sync {
    fn instant(&self) -> Instant;
    fn offset(&self) -> u64;
    fn is_ready(&self) -> bool {
        self.instant().elapsed().as_millis() >= self.offset().into()
    }
}

pub struct MessageQueue<T: TimedMessage> {
    queue: Arc<Mutex<Vec<T>>>,
}

impl<T: TimedMessage> MessageQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn push(&self, message: T) {
        let mut queue = self.queue.lock().await;
        queue.push(message);
    }

    pub async fn push_batch(&self, messages: Vec<T>) {
        let mut queue = self.queue.lock().await;
        queue.extend(messages);
    }

    pub async fn process<F>(&self, mut handler: F)
    where
        F: FnMut(&T) -> bool,
    {
        let mut queue = self.queue.lock().await;
        queue.retain(|msg| {
            if msg.is_ready() {
                handler(msg)
            } else {
                true
            }
        });
    }

    pub fn clone_handle(&self) -> Self {
        Self {
            queue: Arc::clone(&self.queue),
        }
    }
}
```

### 3. MIDI Plugin

**Create `src/plugins/midi/mod.rs`:**

```rust
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

mod commands;
mod processor;
mod state;
mod types;

pub use commands::*;
pub use state::MidiPluginState;
pub use types::*;

use crate::logger::Logger;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("midi")
        .invoke_handler(tauri::generate_handler![commands::send_midi])
        .setup(|app, _api| {
            // Get logger
            let window = app
                .get_webview_window("main")
                .ok_or("Main window not found")?;
            let logger = Logger::new(window);

            // Initialize state
            let state = MidiPluginState::new(logger);

            // Start processor
            processor::start_processor(state.clone());

            // Manage state
            app.manage(state);

            Ok(())
        })
        .build()
}
```

**Create `src/plugins/midi/types.rs`:**

```rust
use serde::Deserialize;
use tokio::time::Instant;
use crate::messaging::queue::TimedMessage;

#[derive(Clone)]
pub struct MidiMessage {
    pub message: Vec<u8>,
    pub instant: Instant,
    pub offset: u64,
    pub port: String,
}

impl TimedMessage for MidiMessage {
    fn instant(&self) -> Instant {
        self.instant
    }

    fn offset(&self) -> u64 {
        self.offset
    }
}

#[derive(Deserialize)]
pub struct MidiMessageFromJS {
    pub message: Vec<u8>,
    pub offset: u64,
    pub requestedport: String,
}

impl MidiMessageFromJS {
    pub fn into_message(self) -> MidiMessage {
        MidiMessage {
            message: self.message,
            instant: Instant::now(),
            offset: self.offset,
            port: self.requestedport,
        }
    }
}
```

**Create `src/plugins/midi/state.rs`:**

```rust
use std::sync::Arc;
use crate::logger::Logger;
use crate::messaging::queue::MessageQueue;
use super::types::MidiMessage;

#[derive(Clone)]
pub struct MidiPluginState {
    pub queue: MessageQueue<MidiMessage>,
    pub logger: Logger,
}

impl MidiPluginState {
    pub fn new(logger: Logger) -> Self {
        Self {
            queue: MessageQueue::new(),
            logger,
        }
    }
}
```

**Create `src/plugins/midi/commands.rs`:**

```rust
use tauri::State;
use super::{MidiMessageFromJS, MidiPluginState};

#[tauri::command]
pub async fn send_midi(
    messages: Vec<MidiMessageFromJS>,
    state: State<'_, MidiPluginState>,
) -> Result<(), String> {
    // Validate
    if messages.len() > 1000 {
        return Err("Too many MIDI messages".to_string());
    }

    // Convert and queue
    let midi_messages: Vec<_> = messages
        .into_iter()
        .map(|m| m.into_message())
        .collect();

    state
        .queue
        .push_batch(midi_messages)
        .await;

    Ok(())
}
```

**Create `src/plugins/midi/processor.rs`:**

```rust
use midir::{MidiOutput, MidiOutputConnection};
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use super::state::MidiPluginState;

pub fn start_processor(state: MidiPluginState) {
    tauri::async_runtime::spawn(async move {
        // Initialize MIDI
        let (connections, port_names) = match init_midi_ports(&state).await {
            Ok(result) => result,
            Err(e) => {
                state.logger.log(
                    format!("Failed to initialize MIDI: {}", e),
                    "error".to_string(),
                );
                return;
            }
        };

        // Process messages
        process_messages(state, connections, port_names).await;
    });
}

async fn init_midi_ports(
    state: &MidiPluginState,
) -> Result<(HashMap<String, MidiOutputConnection>, Vec<String>), Box<dyn std::error::Error>> {
    let midiout = MidiOutput::new("strudel")?;
    let out_ports = midiout.ports();

    if out_ports.is_empty() {
        state.logger.log(
            "No MIDI devices found. Connect a device to enable MIDI.".to_string(),
            "warning".to_string(),
        );
        return Ok((HashMap::new(), Vec::new()));
    }

    // Give frontend time to load
    sleep(Duration::from_secs(3));
    state.logger.log(
        format!("Found {} MIDI devices!", out_ports.len()),
        "info".to_string(),
    );

    let mut connections = HashMap::new();
    let mut port_names = Vec::new();

    for i in 0..out_ports.len() {
        match open_port(i, &state.logger) {
            Ok((name, conn)) => {
                state.logger.log(name.clone(), "info".to_string());
                port_names.push(name.clone());
                connections.insert(name, conn);
            }
            Err(e) => {
                state.logger.log(
                    format!("Failed to open MIDI port {}: {}", i, e),
                    "error".to_string(),
                );
            }
        }
    }

    Ok((connections, port_names))
}

fn open_port(
    index: usize,
    logger: &crate::logger::Logger,
) -> Result<(String, MidiOutputConnection), Box<dyn std::error::Error>> {
    let midiout = MidiOutput::new("strudel")?;
    let ports = midiout.ports();
    let port = ports.get(index).ok_or("Port index out of bounds")?;
    let port_name = midiout.port_name(port)?;
    let connection = midiout.connect(port, &port_name)?;
    Ok((port_name, connection))
}

async fn process_messages(
    state: MidiPluginState,
    mut connections: HashMap<String, MidiOutputConnection>,
    port_names: Vec<String>,
) {
    loop {
        state.queue.process(|msg| {
            let mut conn = connections.get_mut(&msg.port);

            // Support partial name matching
            if conn.is_none() {
                if let Some(key) = port_names.iter().find(|name| name.contains(&msg.port)) {
                    conn = connections.get_mut(key);
                }
            }

            match conn {
                Some(connection) => {
                    if let Err(e) = connection.send(&msg.message) {
                        state.logger.log(
                            format!("MIDI send error: {}", e),
                            "error".to_string(),
                        );
                    }
                }
                None => {
                    state.logger.log(
                        format!("MIDI device not found: {}", msg.port),
                        "warning".to_string(),
                    );
                }
            }

            false // Remove from queue
        }).await;

        sleep(Duration::from_millis(1));
    }
}
```

### 4. Similar Structure for OSC Plugin

Follow the same pattern for the OSC plugin in `src/plugins/osc/`.

### 5. Update Main.rs

**Simplified `src/main.rs`:**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod error;
mod logger;
mod messaging;
mod plugins;

use plugins::{midi, osc};

fn main() {
    #[cfg(debug_assertions)]
    let devtools = tauri_plugin_devtools::init();

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(midi::init())
        .plugin(osc::init());

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(devtools);
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Benefits

1. **Better Organization**: Clear separation of concerns
2. **Reusability**: Shared abstractions reduce duplication
3. **Testability**: Each component can be tested independently
4. **Maintainability**: Easier to understand and modify
5. **Scalability**: Easy to add new plugins
6. **Type Safety**: Better type definitions
7. **Error Handling**: Centralized error types
8. **Plugin Pattern**: Follows Tauri best practices

## Migration Strategy

1. Create new plugin structure alongside existing code
2. Implement MIDI plugin first, test thoroughly
3. Implement OSC plugin
4. Update main.rs to use plugins
5. Remove old bridge modules
6. Update frontend if command names changed

## Implementation Checklist

- [ ] Create directory structure
- [ ] Implement shared error types
- [ ] Implement shared message queue
- [ ] Refactor MIDI as plugin
- [ ] Refactor OSC as plugin
- [ ] Update main.rs
- [ ] Remove old bridge modules
- [ ] Update tests
- [ ] Update documentation
- [ ] Verify MIDI functionality
- [ ] Verify OSC functionality

## Testing

Each plugin should have its own test suite:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_validation() {
        // Test message validation logic
    }

    #[tokio::test]
    async fn test_message_queue() {
        // Test message queuing
    }
}
```
