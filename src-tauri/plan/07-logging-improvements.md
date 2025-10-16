# Logging Improvements

## Overview

Replace the current basic logging system with a structured, leveled logging solution using Rust's standard logging ecosystem.

## Current Issues

1. **No Log Levels**: All messages treated equally
2. **Console-Only**: Logs only go to stdout and frontend
3. **No Structure**: Just string concatenation
4. **No Filtering**: Can't control verbosity
5. **No Persistence**: Logs not saved to disk
6. **No Rotation**: No log file management

## Current Implementation

**`loggerbridge.rs`:**

```rust
impl Logger {
    pub fn log(&self, message: String, message_type: String) {
        println!("{}", message);  // Basic console output
        let _ = self.window.emit_to(
            "main",
            "log-event",
            LoggerPayload {
                message,
                message_type,
            },
        );
    }
}
```

## Proposed Solution

### 1. Use Standard Logging Crates

**Add to `Cargo.toml`:**

```toml
[dependencies]
log = "0.4"
env_logger = "0.11"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### 2. Structured Logging with Tracing

**Update `src/logger.rs`:**

```rust
use std::sync::Arc;
use tauri::{Emitter, WebviewWindow};
use tracing::{debug, error, info, warn};

#[derive(Clone, serde::Serialize)]
pub struct LogEvent {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<serde_json::Value>,
}

#[derive(Clone)]
pub struct Logger {
    window: Arc<WebviewWindow>,
}

impl Logger {
    pub fn new(window: WebviewWindow) -> Self {
        Self {
            window: Arc::new(window),
        }
    }

    pub fn debug(&self, message: impl Into<String>) {
        let msg = message.into();
        debug!("{}", msg);
        self.emit_log("debug", msg, None);
    }

    pub fn info(&self, message: impl Into<String>) {
        let msg = message.into();
        info!("{}", msg);
        self.emit_log("info", msg, None);
    }

    pub fn warn(&self, message: impl Into<String>) {
        let msg = message.into();
        warn!("{}", msg);
        self.emit_log("warning", msg, None);
    }

    pub fn error(&self, message: impl Into<String>) {
        let msg = message.into();
        error!("{}", msg);
        self.emit_log("error", msg, None);
    }

    pub fn log_with_fields(&self, level: &str, message: impl Into<String>, fields: serde_json::Value) {
        let msg = message.into();
        match level {
            "debug" => debug!("{}", msg),
            "info" => info!("{}", msg),
            "warn" => warn!("{}", msg),
            "error" => error!("{}", msg),
            _ => info!("{}", msg),
        }
        self.emit_log(level, msg, Some(fields));
    }

    fn emit_log(&self, level: &str, message: String, fields: Option<serde_json::Value>) {
        let event = LogEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level: level.to_string(),
            target: "strudel".to_string(),
            message,
            fields,
        };

        let _ = self.window.emit("log-event", event);
    }
}
```

### 3. Initialize Logging in Main

**Update `src/main.rs`:**

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

fn setup_logging(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Get log directory
    let log_dir = app.path().app_log_dir()?;
    std::fs::create_dir_all(&log_dir)?;

    // Create file appender with daily rotation
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        log_dir,
        "strudel.log"
    );

    // Create multi-layer subscriber
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            #[cfg(debug_assertions)]
            let default_level = "debug";
            #[cfg(not(debug_assertions))]
            let default_level = "info";

            EnvFilter::new(format!("strudel={},tauri=info", default_level))
        });

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_target(true);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();

    info!("Logging initialized");
    info!("Log directory: {:?}", log_dir);

    Ok(())
}

fn main() {
    // ... builder setup ...

    builder
        .setup(|app| {
            // Initialize logging first
            if let Err(e) = setup_logging(app.handle()) {
                eprintln!("Failed to initialize logging: {}", e);
            }

            info!("Starting Strudel Desktop");

            // ... rest of setup ...
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 4. Structured Logging in Plugins

**Example usage in MIDI plugin:**

```rust
use tracing::{debug, error, info, instrument, warn};

#[instrument(skip(state))]
pub async fn send_midi(
    messages: Vec<MidiMessageFromJS>,
    state: State<'_, MidiPluginState>,
) -> Result<(), String> {
    debug!(count = messages.len(), "Received MIDI messages");

    if messages.len() > 1000 {
        warn!(count = messages.len(), "Large MIDI message batch");
        return Err("Too many MIDI messages".to_string());
    }

    // Process...
    info!(count = messages.len(), "MIDI messages queued");
    Ok(())
}

#[instrument(skip(logger))]
fn open_midi_port(
    index: usize,
    logger: &Logger,
) -> Result<(String, MidiOutputConnection), Box<dyn std::error::Error>> {
    debug!(port_index = index, "Opening MIDI port");

    let midiout = MidiOutput::new("strudel")?;
    let ports = midiout.ports();
    let port = ports.get(index).ok_or("Port index out of bounds")?;
    let port_name = midiout.port_name(port)?;

    info!(port_name = %port_name, "MIDI port opened");

    let connection = midiout.connect(port, &port_name)?;
    Ok((port_name, connection))
}
```

### 5. Log Configuration Command

**Add command to change log level at runtime:**

```rust
use tracing::Level;

#[tauri::command]
pub fn set_log_level(level: String) -> Result<(), String> {
    let level = match level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => return Err("Invalid log level".to_string()),
    };

    // Note: Changing level at runtime requires additional setup
    // with tracing-subscriber reload layer
    Ok(())
}

#[tauri::command]
pub fn get_log_file_path(app: tauri::AppHandle) -> Result<String, String> {
    let log_dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
    Ok(log_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_log_folder(app: tauri::AppHandle) -> Result<(), String> {
    let log_dir = app.path().app_log_dir().map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(log_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(log_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(log_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
```

### 6. Log Viewer UI Integration

**Frontend TypeScript:**

```typescript
import { listen } from '@tauri-apps/api/event';

interface LogEvent {
  timestamp: string;
  level: string;
  target: string;
  message: string;
  fields?: Record<string, any>;
}

export function setupLogListener(callback: (event: LogEvent) => void) {
  return listen<LogEvent>('log-event', (event) => {
    callback(event.payload);
  });
}

// Example usage in React
function LogViewer() {
  const [logs, setLogs] = useState<LogEvent[]>([]);

  useEffect(() => {
    const unlisten = setupLogListener((log) => {
      setLogs(prev => [...prev.slice(-99), log]); // Keep last 100
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);

  return (
    <div className="log-viewer">
      {logs.map((log, i) => (
        <div key={i} className={`log-${log.level}`}>
          <span className="log-time">{log.timestamp}</span>
          <span className="log-level">{log.level}</span>
          <span className="log-message">{log.message}</span>
        </div>
      ))}
    </div>
  );
}
```

### 7. Performance Metrics Logging

**Add span tracking for performance:**

```rust
use tracing::instrument;
use std::time::Instant;

#[instrument(skip(state))]
pub async fn send_midi(
    messages: Vec<MidiMessageFromJS>,
    state: State<'_, MidiPluginState>,
) -> Result<(), String> {
    let start = Instant::now();

    // Process messages...

    let duration = start.elapsed();
    if duration.as_millis() > 10 {
        warn!(
            duration_ms = duration.as_millis(),
            count = messages.len(),
            "Slow MIDI processing"
        );
    }

    debug!(
        duration_ms = duration.as_millis(),
        count = messages.len(),
        "MIDI processing completed"
    );

    Ok(())
}
```

### 8. Panic Hook

**Capture panics in logs:**

```rust
fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info.payload();
        let payload_str = if let Some(s) = payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };

        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "Unknown location".to_string());

        error!(
            payload = %payload_str,
            location = %location,
            "Application panicked"
        );
    }));
}

fn main() {
    setup_panic_hook();
    // ... rest of main
}
```

## Environment Variables

Control logging via environment variables:

```bash
# Set log level
RUST_LOG=debug cargo tauri dev

# Filter by target
RUST_LOG=strudel::midi=debug,strudel::osc=info cargo tauri dev

# Production
RUST_LOG=info cargo tauri build
```

## Benefits

1. **Structured Data**: JSON-formatted logs with fields
2. **Multiple Outputs**: Console, file, and frontend simultaneously
3. **Log Levels**: Control verbosity (trace, debug, info, warn, error)
4. **Filtering**: Target-specific log levels
5. **Rotation**: Automatic daily log rotation
6. **Performance**: Async logging with minimal overhead
7. **Debugging**: Span tracking for distributed tracing
8. **Production Ready**: Persistent logs for debugging issues

## Implementation Checklist

- [ ] Add tracing dependencies to Cargo.toml
- [ ] Implement new Logger struct
- [ ] Set up tracing subscriber in main.rs
- [ ] Configure file appender with rotation
- [ ] Update all logging calls throughout codebase
- [ ] Add log level commands
- [ ] Add panic hook
- [ ] Test log rotation
- [ ] Verify frontend log events
- [ ] Document logging configuration

## Testing

- [ ] Verify logs appear in console
- [ ] Verify logs written to file
- [ ] Verify log rotation works
- [ ] Test log level filtering
- [ ] Check log file size limits
- [ ] Verify frontend receives log events
- [ ] Test panic logging
- [ ] Verify structured fields are captured
