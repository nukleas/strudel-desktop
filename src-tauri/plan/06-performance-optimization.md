# Performance Optimization

## Overview

Optimize message processing, reduce CPU usage, and improve overall performance of the Tauri app.

## Current Performance Issues

### 1. Busy-Wait Loop with `sleep(1ms)`

**Problem:**

Both MIDI and OSC processors use `sleep(Duration::from_millis(1))` in tight loops:

```rust
loop {
    let mut message_queue = message_queue.lock().await;
    message_queue.retain(|message| {
        // process message
    });
    sleep(Duration::from_millis(1));  // ❌ Busy-waiting
}
```

**Issues:**
- Wastes CPU cycles even when idle
- 1ms sleep is not guaranteed (OS scheduler dependent)
- Not truly async - blocks the thread
- Can cause jitter in timing

### 2. Channel Buffer Size

**Problem:**

```rust
let (async_input_transmitter_midi, async_input_receiver_midi) = mpsc::channel(1);
```

**Issues:**
- Buffer size of 1 means sender blocks if receiver is slow
- Can cause dropped messages under high load
- No backpressure handling

### 3. Lock Contention

**Problem:**

Message queue locks are held while iterating and processing:

```rust
let mut message_queue = message_queue_clone.lock().await;
message_queue.retain(|message| {
    // Long-running message processing happens while lock is held
});
```

**Issues:**
- Other tasks blocked while processing
- Can cause delays in message enqueueing

### 4. Redundant Port Lookups

**Problem:**

Every message performs a HashMap lookup and potential string matching:

```rust
let mut out_con = output_connections.get_mut(&message.requestedport);
if out_con.is_none() {
    let key = port_names.iter().find(|port_name| {
        return port_name.contains(&message.requestedport);
    });
    // ...
}
```

## Proposed Solutions

### 1. Replace Busy-Wait with Event-Driven Architecture

**Use Tokio's notify pattern:**

```rust
use tokio::sync::Notify;
use std::sync::Arc;

pub struct MessageQueue<T: TimedMessage> {
    queue: Arc<Mutex<Vec<T>>>,
    notify: Arc<Notify>,
}

impl<T: TimedMessage> MessageQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
            notify: Arc::new(Notify::new()),
        }
    }

    pub async fn push(&self, message: T) {
        let mut queue = self.queue.lock().await;
        queue.push(message);
        drop(queue);  // Release lock before notify
        self.notify.notify_one();
    }

    pub async fn push_batch(&self, messages: Vec<T>) {
        let mut queue = self.queue.lock().await;
        queue.extend(messages);
        drop(queue);
        self.notify.notify_one();
    }
}
```

**Updated processing loop:**

```rust
loop {
    // Wait for notification or timeout
    tokio::select! {
        _ = notify.notified() => {
            // Process messages
        }
        _ = tokio::time::sleep(Duration::from_millis(5)) => {
            // Periodic check for timed messages
        }
    }

    process_ready_messages(&message_queue, &mut connections, &port_names, &logger).await;
}
```

### 2. Increase Channel Buffer Sizes

```rust
// More appropriate buffer sizes
let (async_input_transmitter_midi, async_input_receiver_midi) = mpsc::channel(100);
let (async_output_transmitter_midi, async_output_receiver_midi) = mpsc::channel(100);
let (async_input_transmitter_osc, async_input_receiver_osc) = mpsc::channel(100);
let (async_output_transmitter_osc, async_output_receiver_osc) = mpsc::channel(100);
```

**Or use unbounded channels for better throughput:**

```rust
use tokio::sync::mpsc;

let (tx, rx) = mpsc::unbounded_channel();
```

### 3. Minimize Lock Hold Time

**Extract messages before processing:**

```rust
async fn process_ready_messages(
    queue: &Arc<Mutex<Vec<MidiMessage>>>,
    connections: &mut HashMap<String, MidiOutputConnection>,
    port_names: &[String],
    logger: &Logger,
) {
    // Extract ready messages
    let ready_messages = {
        let mut queue = queue.lock().await;
        let (ready, not_ready): (Vec<_>, Vec<_>) = queue
            .drain(..)
            .partition(|msg| msg.is_ready());
        *queue = not_ready;
        ready
    }; // Lock released here

    // Process without holding lock
    for message in ready_messages {
        send_midi_message(&message, connections, port_names, logger);
    }
}

fn send_midi_message(
    message: &MidiMessage,
    connections: &mut HashMap<String, MidiOutputConnection>,
    port_names: &[String],
    logger: &Logger,
) {
    // Message sending logic
}
```

### 4. Port Name Caching

**Cache port lookups:**

```rust
use std::collections::HashMap;

struct PortCache {
    // Map from partial name to full name
    cache: HashMap<String, String>,
}

impl PortCache {
    fn new(port_names: &[String]) -> Self {
        let mut cache = HashMap::new();
        for full_name in port_names {
            // Cache the full name
            cache.insert(full_name.clone(), full_name.clone());

            // Also cache common substrings
            // e.g., "IAC Driver Bus 1" -> "bus 1" -> "IAC Driver Bus 1"
            if let Some(pos) = full_name.to_lowercase().rfind("bus") {
                let partial = full_name[pos..].to_lowercase();
                cache.entry(partial).or_insert_with(|| full_name.clone());
            }
        }
        cache
    }

    fn resolve(&self, requested: &str, port_names: &[String]) -> Option<String> {
        // Try cache first
        if let Some(full_name) = self.cache.get(requested) {
            return Some(full_name.clone());
        }

        // Fall back to linear search
        port_names
            .iter()
            .find(|name| name.contains(requested))
            .cloned()
    }
}
```

### 5. Batch Processing

**Process messages in batches:**

```rust
const BATCH_SIZE: usize = 32;

async fn process_messages_batch(
    queue: &Arc<Mutex<Vec<MidiMessage>>>,
    connections: &mut HashMap<String, MidiOutputConnection>,
) {
    let messages = {
        let mut queue = queue.lock().await;
        let batch_size = BATCH_SIZE.min(queue.len());
        queue.drain(..batch_size).collect::<Vec<_>>()
    };

    for msg in messages {
        if msg.is_ready() {
            // Process immediately
            send_midi_message(&msg, connections);
        } else {
            // Re-queue if not ready
            let mut queue = queue.lock().await;
            queue.push(msg);
        }
    }
}
```

### 6. Use Atomic Operations for Simple State

For simple counters and flags, use atomics instead of Mutex:

```rust
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

struct Stats {
    messages_sent: AtomicU64,
    messages_failed: AtomicU64,
    is_running: AtomicBool,
}

impl Stats {
    fn increment_sent(&self) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_failed(&self) {
        self.messages_failed.fetch_add(1, Ordering::Relaxed);
    }
}
```

### 7. Optimize Message Timestamp Checks

**Use a min-heap for time-ordered processing:**

```rust
use std::collections::BinaryHeap;
use std::cmp::Ordering;

#[derive(Eq, PartialEq)]
struct TimedMessageWrapper {
    message: MidiMessage,
    ready_at: Instant,
}

impl Ord for TimedMessageWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.ready_at.cmp(&self.ready_at)
    }
}

impl PartialOrd for TimedMessageWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Usage
let mut heap: BinaryHeap<TimedMessageWrapper> = BinaryHeap::new();

// Process only messages at the top of heap that are ready
while let Some(wrapper) = heap.peek() {
    if wrapper.ready_at > Instant::now() {
        break;  // Not ready yet
    }
    let wrapper = heap.pop().unwrap();
    send_message(wrapper.message);
}
```

### 8. Lazy Initialization

Only initialize MIDI/OSC when first needed:

```rust
use once_cell::sync::Lazy;

static MIDI_INITIALIZED: AtomicBool = AtomicBool::new(false);

async fn ensure_midi_initialized(state: &MidiPluginState) {
    if MIDI_INITIALIZED.load(Ordering::Relaxed) {
        return;
    }

    // Initialize...
    MIDI_INITIALIZED.store(true, Ordering::Relaxed);
}
```

## Benchmarking

Add benchmarking to measure improvements:

```rust
#[cfg(test)]
mod benches {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn bench_message_processing() {
        let queue = MessageQueue::new();

        // Add 1000 messages
        let start = Instant::now();
        for i in 0..1000 {
            queue.push(MidiMessage {
                message: vec![0x90, 60, 100],
                instant: Instant::now(),
                offset: i,
                port: "test".to_string(),
            }).await;
        }
        let duration = start.elapsed();

        println!("Enqueued 1000 messages in {:?}", duration);
        assert!(duration.as_millis() < 100);  // Should be fast
    }
}
```

## Memory Optimization

### 1. Reuse Allocations

```rust
// Instead of creating new vectors constantly
let mut buffer = Vec::with_capacity(1024);

loop {
    buffer.clear();  // Reuse allocation
    // Fill buffer...
}
```

### 2. Message Pooling

```rust
use object_pool::Pool;

lazy_static! {
    static ref MESSAGE_POOL: Pool<Vec<u8>> = Pool::new(100, || Vec::with_capacity(256));
}

// Borrow from pool
let mut buffer = MESSAGE_POOL.pull();
// Use buffer...
// Automatically returned to pool when dropped
```

## Monitoring

Add performance metrics:

```rust
#[tauri::command]
pub async fn get_midi_stats(state: State<'_, MidiPluginState>) -> Result<MidiStats, String> {
    Ok(MidiStats {
        messages_sent: state.stats.messages_sent.load(Ordering::Relaxed),
        messages_failed: state.stats.messages_failed.load(Ordering::Relaxed),
        queue_size: state.queue.len().await,
        active_ports: state.port_count.load(Ordering::Relaxed),
    })
}

#[derive(serde::Serialize)]
pub struct MidiStats {
    messages_sent: u64,
    messages_failed: u64,
    queue_size: usize,
    active_ports: usize,
}
```

## Benefits

1. **Lower CPU Usage**: Event-driven instead of busy-waiting
2. **Better Throughput**: Larger buffers and batch processing
3. **Lower Latency**: Minimized lock contention
4. **More Responsive**: Faster lookups with caching
5. **Scalable**: Can handle higher message rates
6. **Observable**: Built-in performance metrics

## Implementation Priority

1. ✅ High: Replace busy-wait with event-driven architecture
2. ✅ High: Increase channel buffer sizes
3. ✅ High: Minimize lock hold time
4. ✅ Medium: Add port name caching
5. ✅ Medium: Implement batch processing
6. ✅ Low: Add message pooling
7. ✅ Low: Add performance monitoring

## Testing

- [ ] Benchmark message throughput before/after
- [ ] Test with high message rate (>1000 msgs/sec)
- [ ] Monitor CPU usage during idle and active periods
- [ ] Test latency with precision timing
- [ ] Profile with `cargo flamegraph`
- [ ] Test memory usage over extended periods
- [ ] Stress test with multiple simultaneous operations

## Tools

- **Profiling**: `cargo flamegraph`, `perf`, Instruments (macOS)
- **Benchmarking**: `criterion` crate
- **Monitoring**: `tokio-console` for async runtime inspection
