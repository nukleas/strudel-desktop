# Error Handling & Robustness

## Current Issues

The codebase currently uses `.unwrap()` extensively, which causes the application to panic when encountering errors. This leads to poor user experience and difficult debugging.

### Problem Areas

1. **MIDI Port Opening** (`midibridge.rs:52-79`)
   - `let midiout = MidiOutput::new("strudel").unwrap();`
   - `let port = ports.get(i).unwrap();`
   - `let port_name = midiout.port_name(port).unwrap();`
   - `let out_con = midiout.connect(port, &port_name).unwrap();`

2. **OSC Socket Setup** (`oscbridge.rs:58-62`)
   - `let sock = UdpSocket::bind("127.0.0.1:57122").unwrap();`
   - Uses `.expect()` which is slightly better but still panics

3. **Window Access** (`main.rs:37`)
   - `let window = app.get_webview_window("main").unwrap();`

4. **Message Encoding** (`oscbridge.rs:154`)
   - `let msg_buf = encoder::encode(&OscPacket::Bundle(bundle)).unwrap();`

## Proposed Solutions

### 1. Graceful MIDI Initialization

```rust
pub fn init(
    logger: Logger,
    async_input_receiver: mpsc::Receiver<Vec<MidiMessage>>,
    mut async_output_receiver: mpsc::Receiver<Vec<MidiMessage>>,
    async_output_transmitter: mpsc::Sender<Vec<MidiMessage>>
) -> Result<(), Box<dyn std::error::Error>> {
    // Spawn async process
    tauri::async_runtime::spawn(async move {
        async_process_model(async_input_receiver, async_output_transmitter).await
    });

    let message_queue: Arc<Mutex<Vec<MidiMessage>>> = Arc::new(Mutex::new(Vec::new()));

    // Clone for listener
    let message_queue_clone = Arc::clone(&message_queue);
    tauri::async_runtime::spawn(async move {
        loop {
            if let Some(package) = async_output_receiver.recv().await {
                let mut message_queue = message_queue_clone.lock().await;
                for message in package {
                    (*message_queue).push(message);
                }
            }
        }
    });

    // Main MIDI handling with proper error handling
    let message_queue_clone = Arc::clone(&message_queue);
    tauri::async_runtime::spawn(async move {
        match MidiOutput::new("strudel") {
            Ok(midiout) => {
                let out_ports = midiout.ports();

                if out_ports.is_empty() {
                    logger.log(
                        "No MIDI devices found. Connect a device or enable IAC Driver to enable MIDI.".to_string(),
                        "warning".to_string()
                    );
                    return;
                }

                // Give frontend time to load
                sleep(Duration::from_secs(3));
                logger.log(format!("Found {} MIDI devices!", out_ports.len()), "info".to_string());

                // Open all ports with error handling
                let mut output_connections = HashMap::new();
                let mut port_names = Vec::new();

                for i in 0..out_ports.len() {
                    match open_midi_port(i, &logger) {
                        Ok((port_name, connection)) => {
                            port_names.push(port_name.clone());
                            output_connections.insert(port_name, connection);
                        }
                        Err(e) => {
                            logger.log(
                                format!("Failed to open MIDI port {}: {}", i, e),
                                "error".to_string()
                            );
                        }
                    }
                }

                // Process message queue
                process_midi_messages(message_queue_clone, output_connections, port_names, logger).await;
            }
            Err(e) => {
                logger.log(
                    format!("Failed to initialize MIDI: {}", e),
                    "error".to_string()
                );
            }
        }
    });

    Ok(())
}

fn open_midi_port(index: usize, logger: &Logger) -> Result<(String, MidiOutputConnection), Box<dyn std::error::Error>> {
    let midiout = MidiOutput::new("strudel")?;
    let ports = midiout.ports();
    let port = ports.get(index).ok_or("Port index out of bounds")?;
    let port_name = midiout.port_name(port)?;
    logger.log(port_name.clone(), "info".to_string());
    let connection = midiout.connect(port, &port_name)?;
    Ok((port_name, connection))
}

async fn process_midi_messages(
    message_queue: Arc<Mutex<Vec<MidiMessage>>>,
    mut output_connections: HashMap<String, MidiOutputConnection>,
    port_names: Vec<String>,
    logger: Logger
) {
    loop {
        let mut message_queue = message_queue.lock().await;

        message_queue.retain(|message| {
            if message.instant.elapsed().as_millis() < message.offset.into() {
                return true;
            }

            let mut out_con = output_connections.get_mut(&message.requestedport);

            // Partial name matching
            if out_con.is_none() {
                if let Some(key) = port_names.iter().find(|name| name.contains(&message.requestedport)) {
                    out_con = output_connections.get_mut(key);
                }
            }

            if let Some(connection) = out_con {
                if let Err(err) = connection.send(&message.message) {
                    logger.log(format!("MIDI message send error: {}", err), "error".to_string());
                }
            } else {
                logger.log(
                    format!("Failed to find MIDI device: {}", message.requestedport),
                    "warning".to_string()
                );
            }

            false
        });

        sleep(Duration::from_millis(1));
    }
}
```

### 2. Graceful OSC Initialization

```rust
pub fn init(
    logger: Logger,
    async_input_receiver: mpsc::Receiver<Vec<OscMsg>>,
    mut async_output_receiver: mpsc::Receiver<Vec<OscMsg>>,
    async_output_transmitter: mpsc::Sender<Vec<OscMsg>>,
) -> Result<(), Box<dyn std::error::Error>> {
    tauri::async_runtime::spawn(async move {
        async_process_model(async_input_receiver, async_output_transmitter).await
    });

    let message_queue: Arc<Mutex<Vec<OscMsg>>> = Arc::new(Mutex::new(Vec::new()));

    // Message receiver
    let message_queue_clone = Arc::clone(&message_queue);
    tauri::async_runtime::spawn(async move {
        loop {
            if let Some(package) = async_output_receiver.recv().await {
                let mut message_queue = message_queue_clone.lock().await;
                for message in package {
                    (*message_queue).push(message);
                }
            }
        }
    });

    // OSC processing with error handling
    let message_queue_clone = Arc::clone(&message_queue);
    tauri::async_runtime::spawn(async move {
        match UdpSocket::bind("127.0.0.1:57122") {
            Ok(sock) => {
                let to_addr = "127.0.0.1:57120";

                if let Err(e) = sock.set_nonblocking(true) {
                    logger.log(
                        format!("Failed to set OSC socket to non-blocking: {}", e),
                        "error".to_string()
                    );
                    return;
                }

                if let Err(e) = sock.connect(to_addr) {
                    logger.log(
                        format!("Could not connect to OSC address {}: {}", to_addr, e),
                        "error".to_string()
                    );
                    return;
                }

                logger.log(
                    format!("OSC initialized: sending to {}", to_addr),
                    "info".to_string()
                );

                // Process messages
                loop {
                    let mut message_queue = message_queue_clone.lock().await;

                    message_queue.retain(|message| {
                        match sock.send(&message.msg_buf) {
                            Ok(_) => {},
                            Err(e) => {
                                logger.log(
                                    format!("OSC message failed to send: {}. Server might be unavailable.", e),
                                    "warning".to_string()
                                );
                            }
                        }
                        false
                    });

                    sleep(Duration::from_millis(1));
                }
            }
            Err(e) => {
                logger.log(
                    format!("Failed to bind OSC socket on 127.0.0.1:57122: {}", e),
                    "error".to_string()
                );
            }
        }
    });

    Ok(())
}
```

### 3. Proper Window Handling in Setup

```rust
.setup(|app| {
    let window = app.get_webview_window("main")
        .ok_or("Failed to get main window")?;

    let logger = Logger {
        window: Arc::new(window),
    };

    // Initialize with error handling
    if let Err(e) = midibridge::init(
        logger.clone(),
        async_input_receiver_midi,
        async_output_receiver_midi,
        async_output_transmitter_midi,
    ) {
        eprintln!("Warning: MIDI initialization failed: {}", e);
        // Continue anyway - app can work without MIDI
    }

    if let Err(e) = oscbridge::init(
        logger,
        async_input_receiver_osc,
        async_output_receiver_osc,
        async_output_transmitter_osc,
    ) {
        eprintln!("Warning: OSC initialization failed: {}", e);
        // Continue anyway - app can work without OSC
    }

    Ok(())
})
```

### 4. Better Error Types

Create a custom error type for better error handling:

```rust
// src/error.rs
use std::fmt;

#[derive(Debug)]
pub enum StrudelError {
    MidiInit(String),
    MidiPortOpen(String),
    MidiSend(String),
    OscInit(String),
    OscSend(String),
    WindowNotFound(String),
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
        }
    }
}

impl std::error::Error for StrudelError {}
```

## Benefits

1. **No More Panics**: Application continues running even when MIDI/OSC fails
2. **Better User Feedback**: Clear error messages show what went wrong
3. **Easier Debugging**: Errors include context about what failed
4. **Graceful Degradation**: App can work without MIDI/OSC if needed
5. **Production Ready**: Suitable for distribution to end users

## Implementation Priority

1. ✅ High: Replace all `.unwrap()` calls in initialization code
2. ✅ High: Add proper error messages via Logger
3. ✅ Medium: Create custom error types
4. ✅ Medium: Add recovery mechanisms
5. ✅ Low: Add error reporting/telemetry

## Testing Checklist

- [ ] Test app startup with no MIDI devices
- [ ] Test app startup with OSC port already in use
- [ ] Test MIDI device disconnection during use
- [ ] Test OSC server unavailable scenario
- [ ] Test rapid message sending under error conditions
