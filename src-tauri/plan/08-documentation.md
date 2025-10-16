# Documentation Plan

## Overview

Create comprehensive documentation for the Tauri backend, making it easier for contributors and maintainers to understand and modify the codebase.

## Current State

- Minimal inline documentation
- No module-level docs
- No architecture documentation
- No README for Rust backend
- No API documentation

## Proposed Documentation Structure

```
src-tauri/
├── README.md                    # Overview and quick start
├── ARCHITECTURE.md              # Architecture documentation
├── DEVELOPMENT.md               # Development guide
├── API.md                       # Command API reference
├── docs/
│   ├── midi.md                  # MIDI subsystem
│   ├── osc.md                   # OSC subsystem
│   ├── building.md              # Build instructions
│   └── debugging.md             # Debugging guide
└── src/
    └── (inline documentation)
```

## Documentation Files

### 1. README.md

**Create `src-tauri/README.md`:**

```markdown
# Strudel Desktop - Rust Backend

The Tauri-based desktop application for Strudel, adding MIDI and OSC support to the browser-based music live-coding environment.

## Features

- **MIDI Output**: Send MIDI messages to hardware and software synthesizers
- **OSC Support**: Communicate with SuperCollider and other OSC-enabled software
- **Cross-Platform**: Windows, macOS, and Linux support
- **Low Latency**: Direct audio/MIDI communication without browser limitations

## Architecture

The backend is built with:
- **Tauri 2.0**: Cross-platform desktop framework
- **Tokio**: Async runtime for concurrent message processing
- **midir**: Cross-platform MIDI I/O
- **rosc**: OSC protocol implementation

See [ARCHITECTURE.md](./ARCHITECTURE.md) for detailed architecture documentation.

## Quick Start

### Prerequisites

- Rust 1.60+
- Node.js 18+
- pnpm

### Development

```bash
# Install dependencies
pnpm i

# Run in development mode
pnpm tauri:dev

# Build for production
pnpm tauri:build
```

### Environment Variables

- `RUST_LOG`: Control log level (e.g., `RUST_LOG=debug`)
- `STRUDEL_MIDI_BUFFER`: Override MIDI buffer size (default: 100)
- `STRUDEL_OSC_BUFFER`: Override OSC buffer size (default: 100)

## Project Structure

```
src-tauri/
├── src/
│   ├── main.rs              # Application entry point
│   ├── logger.rs            # Logging infrastructure
│   ├── error.rs             # Error types
│   ├── midibridge.rs        # MIDI subsystem
│   ├── oscbridge.rs         # OSC subsystem
│   └── loggerbridge.rs      # Logger bridge
├── Cargo.toml               # Rust dependencies
├── tauri.conf.json          # Tauri configuration
└── build.rs                 # Build script
```

## Commands

The backend exposes these commands to the frontend:

### MIDI Commands

- `sendmidi`: Send MIDI messages

### OSC Commands

- `sendosc`: Send OSC messages

See [API.md](./API.md) for detailed API documentation.

## Contributing

See [DEVELOPMENT.md](./DEVELOPMENT.md) for development guidelines.

## License

AGPL-3.0-or-later
```

### 2. ARCHITECTURE.md

**Create `src-tauri/ARCHITECTURE.md`:**

```markdown
# Architecture Documentation

## Overview

The Strudel desktop application uses Tauri to bridge web technologies with native system capabilities, specifically MIDI and OSC communication.

## System Architecture

```
┌─────────────────────────────────────────┐
│          Frontend (Browser)              │
│         React + Web Audio API            │
└───────────────┬─────────────────────────┘
                │ IPC (Tauri Commands)
┌───────────────▼─────────────────────────┐
│           Tauri Runtime                  │
│  (Rust Backend + WebView Management)     │
└───────────┬────────────┬────────────────┘
            │            │
    ┌───────▼──────┐ ┌──▼──────────┐
    │ MIDI Bridge  │ │ OSC Bridge  │
    └───────┬──────┘ └──┬──────────┘
            │            │
    ┌───────▼──────┐ ┌──▼──────────┐
    │ midir        │ │ UDP Socket  │
    │ (MIDI I/O)   │ │ (OSC)       │
    └───────┬──────┘ └──┬──────────┘
            │            │
    ┌───────▼──────┐ ┌──▼──────────┐
    │ OS MIDI      │ │ SuperCollider│
    │ Drivers      │ │ / OSC Server │
    └──────────────┘ └─────────────┘
```

## Component Details

### Frontend Layer

- **Technology**: React, TypeScript, Web Audio API
- **Responsibility**: User interface, pattern evaluation, audio synthesis
- **Communication**: Calls Rust backend via Tauri IPC

### Tauri Runtime

- **Technology**: Rust, Tokio async runtime
- **Responsibility**:
  - Window management
  - IPC handling
  - Plugin lifecycle
  - State management

### MIDI Bridge

**File**: `src/midibridge.rs`

**Flow**:
1. Frontend sends MIDI messages via `sendmidi` command
2. Messages enter input channel (mpsc)
3. Pass through async processing pipeline
4. Queued in time-ordered message queue
5. Processed by background task
6. Sent to MIDI output ports via midir

**Key Components**:
- `MidiMessage`: Timed message structure
- `AsyncInputTransmit`: Shared state for channel communication
- `sendmidi`: Tauri command handler
- `init`: Initialization and processor spawning
- Message queue with timing logic

**Timing**:
- Uses Tokio `Instant` for precise timing
- Messages include offset for scheduled delivery
- 1ms polling loop (TODO: optimize to event-driven)

### OSC Bridge

**File**: `src/oscbridge.rs`

**Flow**:
1. Frontend sends OSC messages via `sendosc` command
2. Messages converted to OSC bundles with timetags
3. Queued for delivery
4. Sent via UDP to OSC server (default: 127.0.0.1:57120)

**Key Components**:
- `OscMsg`: OSC message wrapper
- `AsyncInputTransmit`: Shared state
- `sendosc`: Command handler
- OSC bundle encoding with timetags

**Protocol**:
- Uses OSC 1.0 protocol
- Bundles include NTP timetags
- Sends to localhost by default (SuperCollider)

### Logger Bridge

**File**: `src/loggerbridge.rs`

**Responsibility**:
- Bridge between Rust logging and frontend UI
- Emits log events to frontend via Tauri events
- Console logging for development

## Message Flow

### MIDI Message Flow

```
Frontend                 Rust Backend
   │                         │
   │ invoke('sendmidi')      │
   ├────────────────────────►│
   │                         │ Validate
   │                         │ Convert to MidiMessage
   │                         │ Add to channel
   │                         ▼
   │                    Channel receiver
   │                         │
   │                         ▼
   │                    Message queue
   │                         │
   │                    ┌────┴────┐
   │                    │ Timer   │
   │                    │ Check   │
   │                    └────┬────┘
   │                         │
   │                         ▼
   │                    Send to MIDI port
   │                         │
   │◄────────────────────────┤ (log event)
   │                         │
```

### OSC Message Flow

```
Frontend                 Rust Backend
   │                         │
   │ invoke('sendosc')       │
   ├────────────────────────►│
   │                         │ Convert params
   │                         │ Create OSC bundle
   │                         │ Encode message
   │                         │ Add to channel
   │                         ▼
   │                    Channel receiver
   │                         │
   │                         ▼
   │                    Message queue
   │                         │
   │                         ▼
   │                    Send via UDP
   │                         │
   │◄────────────────────────┤ (log event)
   │                         │
```

## Concurrency Model

### Async Runtime

- **Runtime**: Tokio
- **Executor**: Multi-threaded work-stealing scheduler
- **Tasks**: Separate tasks for MIDI processing, OSC processing, and IPC handling

### State Management

- **Channels**: mpsc channels for message passing
- **Mutexes**: Tokio async mutexes for shared state
- **Message Queues**: Arc<Mutex<Vec<T>>> for concurrent access

### Threading

- Main thread: Tauri event loop and WebView
- Background tasks: MIDI/OSC processing (Tokio runtime)
- No manual thread spawning (all via Tokio)

## Error Handling

Current approach:
- `.unwrap()` used extensively (TODO: improve)
- Errors logged to console and frontend
- App continues running even if MIDI/OSC fails

Planned improvements:
- Custom error types
- Proper Result propagation
- Graceful degradation

## Performance Considerations

### Current Bottlenecks

1. **1ms polling loop**: CPU overhead even when idle
2. **Lock contention**: Queue locks held during processing
3. **Channel buffer size**: Size of 1 can cause backpressure

### Optimization Opportunities

1. Event-driven architecture (replace polling)
2. Batch processing
3. Larger channel buffers
4. Lock-free data structures

See [plan/06-performance-optimization.md](./plan/06-performance-optimization.md) for details.

## Security

- CSP configured (currently permissive)
- Filesystem access scoped
- No external network access (only localhost)
- Input validation needed for commands

## Testing Strategy

- Unit tests for message processing
- Integration tests for MIDI/OSC flow
- Manual testing with real MIDI devices
- Performance benchmarks

## Future Enhancements

- MIDI input support
- Multiple OSC targets
- Plugin architecture
- Configuration UI
- Device hot-plugging
- Better error recovery
```

### 3. API.md

**Create `src-tauri/API.md`:**

```markdown
# Backend API Reference

## Commands

Commands are callable from the frontend using Tauri's `invoke` function.

### MIDI Commands

#### `sendmidi`

Send MIDI messages to connected output devices.

**Signature**:
```typescript
function sendmidi(messages: MidiMessage[]): Promise<void>
```

**Parameters**:

```typescript
interface MidiMessage {
  message: number[];      // MIDI message bytes (e.g., [0x90, 60, 100])
  offset: number;         // Delay in milliseconds before sending
  requestedport: string;  // Target MIDI port name (partial match supported)
}
```

**Example**:
```typescript
import { invoke } from '@tauri-apps/api/core';

await invoke('sendmidi', {
  messagesfromjs: [
    {
      message: [0x90, 60, 100],  // Note On, C4, velocity 100
      offset: 0,                  // Send immediately
      requestedport: 'IAC Driver Bus 1'
    },
    {
      message: [0x80, 60, 0],     // Note Off, C4
      offset: 500,                 // Send after 500ms
      requestedport: 'IAC Driver Bus 1'
    }
  ]
});
```

**Port Name Matching**:
- Exact match: `"IAC Driver Bus 1"`
- Partial match: `"bus 1"` (case-insensitive)
- First matching port is used if multiple matches exist

**Errors**:
- Too many messages (>1000)
- Invalid MIDI port
- MIDI send failure

---

#### `sendosc`

Send OSC messages to an OSC server (default: localhost:57120).

**Signature**:
```typescript
function sendosc(messages: OscMessage[]): Promise<void>
```

**Parameters**:

```typescript
interface OscMessage {
  target: string;           // OSC address (e.g., "/s_new")
  timestamp: number;        // Unix timestamp for scheduled delivery
  params: OscParam[];       // OSC arguments
}

interface OscParam {
  name: string;             // Parameter name
  value: string;            // Parameter value (as string)
  valueisnumber: boolean;   // If true, parse value as float
}
```

**Example**:
```typescript
import { invoke } from '@tauri-apps/api/core';

await invoke('sendosc', {
  messagesfromjs: [
    {
      target: '/s_new',
      timestamp: Date.now() / 1000,
      params: [
        { name: 'defName', value: 'sine', valueisnumber: false },
        { name: 'freq', value: '440', valueisnumber: true },
        { name: 'amp', value: '0.5', valueisnumber: true }
      ]
    }
  ]
});
```

**OSC Bundle Format**:
- Messages are wrapped in OSC bundles
- Includes NTP timetag for scheduling
- Sent via UDP to 127.0.0.1:57120

**Errors**:
- OSC encoding failure
- Network send failure
- Invalid timestamp

---

## Events

Events are emitted from the backend and can be listened to in the frontend.

### `log-event`

Emitted when the backend logs a message.

**Payload**:
```typescript
interface LogEvent {
  message: string;       // Log message
  message_type: string;  // Message type: '', 'error', 'warning', 'info'
}
```

**Example**:
```typescript
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen<LogEvent>('log-event', (event) => {
  console.log(`[${event.payload.message_type}] ${event.payload.message}`);
});

// Later: unlisten();
```

---

## Configuration

### Environment Variables

- `RUST_LOG`: Set log level (e.g., `RUST_LOG=debug`)

### Tauri Config

See `tauri.conf.json` for application configuration.

---

## Error Handling

All commands return `Promise<void>` or `Promise<T>`.

Errors are thrown as strings:
```typescript
try {
  await invoke('sendmidi', { messagesfromjs: messages });
} catch (error) {
  console.error('MIDI error:', error);
}
```
```

### 4. Inline Documentation

**Add comprehensive rustdoc comments:**

```rust
/// MIDI bridge module for sending MIDI messages to hardware and software devices.
///
/// This module provides async MIDI output capabilities using the `midir` crate.
/// Messages are queued and processed with precise timing using Tokio.
///
/// # Architecture
///
/// ```text
/// Frontend --invoke--> sendmidi --channel--> Message Queue --timer--> MIDI Output
/// ```
///
/// # Example
///
/// ```no_run
/// use crate::midibridge::{init, AsyncInputTransmit};
///
/// // Initialize MIDI subsystem
/// init(logger, rx, rx2, tx);
/// ```

/// Represents a MIDI message with timing information.
///
/// # Fields
///
/// * `message` - Raw MIDI bytes (e.g., `[0x90, 60, 100]` for Note On)
/// * `instant` - When the message was created (for offset calculation)
/// * `offset` - Milliseconds to wait before sending
/// * `requestedport` - Target MIDI port name (supports partial matching)
#[derive(Clone)]
pub struct MidiMessage {
    pub message: Vec<u8>,
    pub instant: Instant,
    pub offset: u64,
    pub requestedport: String,
}

/// Tauri command to send MIDI messages from the frontend.
///
/// # Arguments
///
/// * `messagesfromjs` - Array of MIDI messages to send
/// * `state` - Managed state containing the message channel
///
/// # Returns
///
/// `Ok(())` on success, `Err(String)` on failure
///
/// # Errors
///
/// Returns an error if:
/// - Too many messages (>1000) in a single batch
/// - Channel send fails
///
/// # Example
///
/// ```javascript
/// await invoke('sendmidi', {
///   messagesfromjs: [
///     { message: [0x90, 60, 100], offset: 0, requestedport: 'IAC Driver' }
///   ]
/// });
/// ```
#[tauri::command]
pub async fn sendmidi(
    messagesfromjs: Vec<MessageFromJS>,
    state: tauri::State<'_, AsyncInputTransmit>
) -> Result<(), String> {
    // Implementation...
}
```

## Benefits

1. **Onboarding**: New contributors can understand codebase quickly
2. **Maintenance**: Easier to modify and debug
3. **Collaboration**: Clear interfaces and expectations
4. **Quality**: Encourages better design decisions
5. **Reference**: Generated rustdoc for API reference

## Implementation Checklist

- [ ] Create README.md
- [ ] Create ARCHITECTURE.md
- [ ] Create API.md
- [ ] Create DEVELOPMENT.md
- [ ] Add inline rustdoc comments to all public items
- [ ] Document all modules
- [ ] Add examples to documentation
- [ ] Generate rustdoc: `cargo doc --open`
- [ ] Review and refine documentation
- [ ] Add diagrams (using mermaid or ASCII)

## Tools

- **rustdoc**: Generate HTML documentation from inline comments
- **cargo-readme**: Generate README from lib.rs docs
- **mdBook**: For comprehensive documentation books
- **Mermaid**: For diagrams in markdown
