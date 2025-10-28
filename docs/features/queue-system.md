# Queue System - Progressive Pattern Building

## Overview

The Queue System enables progressive pattern building in Strudel Desktop's Chat Assistant. Instead of applying all changes at once, the AI can queue multiple edits that apply automatically after specified time intervals, creating smooth musical progressions.

**Key Features:**
- 🎬 **Progressive Building**: Build patterns step-by-step with musical pacing
- 🤖 **Tool-Based**: Uses native Rust tool calls (not text parsing)
- ⏱️ **Auto-Advance**: Changes apply automatically after wait cycles
- 🎛️ **Manual Control**: Skip, preview, or apply changes manually
- 🎵 **Musical Timing**: Uses Strudel's cycle counter for accurate timing

---

## Quick Start

### For Users

1. **Enable Queue Mode**: Click the 🎬 button in the chat header
2. **Request Progressive Changes**:
   ```
   "Build a techno beat progressively"
   ```
3. **Watch It Build**: Changes apply automatically with musical timing
4. **Manual Controls** (optional):
   - **Apply Next** - Skip waiting and apply now
   - **Skip** - Skip the current change
   - **Apply All** - Apply everything immediately
   - **Preview** - See code before it applies
   - **Clear** - Remove all pending changes

### For the AI Agent

When Queue Mode is active, call the `apply_live_code_edit` tool multiple times with these parameters:

```javascript
apply_live_code_edit({
  mode: "replace",           // or "append"
  code: "s('bd*4').gain(0.8)",
  description: "Add kick drum",  // What this change does
  wait_cycles: 0                 // Cycles to wait before applying
})
```

**Best Practice**: Make 2-3 substantial calls per response
- Combine related elements (kick + snare + hats = one call)
- Use wait_cycles: 0 for first, 8-16 for subsequent changes
- Don't make too many small changes

---

## Architecture

### Backend (Rust)

**Tool Definition** (`src-tauri/src/tools.rs`):
```rust
apply_live_code_edit {
  mode: "replace" | "append",
  code: String,
  description: Option<String>,   // Queue: change description
  wait_cycles: Option<u32>        // Queue: cycles to wait
}
```

**Behavior:**
- If `description` or `wait_cycles` is provided → Queue the change
- If both omitted → Apply immediately (backward compatible)

**Event Emitted:**
```rust
// Event: chat-queue-edit
{
  mode: LiveEditMode,
  code: String,
  description: String,
  wait_cycles: u32
}
```

### Frontend (React)

**State Management** (`useReplContext.jsx`):
```javascript
const [changeQueue, setChangeQueue] = useState([])
const [queueEnabled, setQueueEnabled] = useState(false)
const [cyclesSinceLastChange, setCyclesSinceLastChange] = useState(0)
```

**Event Listener** (`ChatTab.jsx`):
```javascript
listen('chat-queue-edit', ({ payload }) => {
  const queueItem = {
    description: payload.description,
    code: payload.code,
    mode: payload.mode,
    waitCycles: payload.wait_cycles,
    autoEvaluate: true
  }
  context.addToQueue(queueItem)
})
```

**Auto-Advance Logic**:
```javascript
useEffect(() => {
  if (!queueEnabled || changeQueue.length === 0) return

  const nextChange = changeQueue[0]
  const waitCycles = nextChange.waitCycles || 0

  // Auto-apply when ready
  if (cyclesSinceLastChange >= waitCycles) {
    applyNextChange()
  }
}, [cyclesSinceLastChange, changeQueue, queueEnabled])
```

---

## Cycle Tracking

Uses Strudel's native scheduler for accurate timing:

```javascript
const interval = setInterval(() => {
  if (editorRef.current?.repl?.scheduler) {
    const currentCycle = Math.floor(
      editorRef.current.repl.scheduler.now()
    )
    const cyclesElapsed = currentCycle - lastChangeCycle
    setCyclesSinceLastChange(cyclesElapsed)
  }
}, 100) // Check 10x per second
```

**Why `scheduler.now()`?**
- ✅ Accurate even when tempo changes
- ✅ Accounts for Strudel's timing adjustments
- ✅ No drift or accumulation errors
- ✅ Same cycle counter as pattern scheduler

---

## Example Workflow

**User:** "Build a techno beat progressively" [🎬 Queue Mode enabled]

**Agent Response:**
```
I'll build this in 2 major layers:

[Tool Call 1]
apply_live_code_edit({
  mode: "replace",
  code: "stack(s('bd*4'), s('~ sd ~ sd'), s('hh*8')).gain(0.8)",
  description: "Full drum pattern",
  wait_cycles: 0
})

[Tool Call 2]
apply_live_code_edit({
  mode: "replace",
  code: "stack(s('bd*4'), s('~ sd ~ sd'), s('hh*8'), note('c2 ~ eb2 ~').s('sawtooth').lpf(400), note('c4 e4 g4').s('triangle').room(0.5)).gain(0.8)",
  description: "Add bass and melody",
  wait_cycles: 8
})

Drums start now, full arrangement in 8 cycles!
```

**What Happens:**
1. Backend receives 2 tool calls
2. Emits 2 `chat-queue-edit` events
3. Frontend adds both to queue
4. **First change** applies immediately (wait_cycles=0)
5. Cycle counter starts tracking
6. **After 8 cycles**, second change auto-applies
7. Smooth musical progression! 🎵

---

## UI Components

### Queue Panel

Shows:
- **Step Counter**: "Step 1/3"
- **Next Change**: Description and preview
- **Timing**: "Waiting... 2/8 cycles" or "Ready!"
- **Actions**: Apply Next, Skip, Preview, Apply All, Clear
- **Queue List**: Expandable list of all pending changes

### Visual States

```
🟢 Ready to Apply (cycles elapsed >= wait_cycles)
🟡 Waiting... (still counting cycles)
⏸️ Paused (pattern stopped)
```

---

## Benefits

### vs. Immediate Application
- ✅ Musical pacing - patterns breathe between changes
- ✅ User can hear each layer before next addition
- ✅ Live coding friendly - build complexity gradually

### vs. JSON Parsing
- ✅ No JSON syntax errors
- ✅ Type-safe parameters (Rust validates)
- ✅ Clear error messages
- ✅ LLM-friendly (tool use is natural)

### vs. Markdown Parsing
- ✅ No regex complexity
- ✅ No parsing ambiguity
- ✅ Strongly typed
- ✅ Better error handling

### vs. Manual Clicking
- ✅ Auto-applies after wait period
- ✅ Smooth musical flow
- ✅ Still allows manual override
- ✅ User controls pacing

---

## Agent Guidelines

### ✅ Good Chunking (2-3 calls)

```javascript
// Call 1: Complete drum section
apply_live_code_edit({
  code: "stack(s('bd*4'), s('~ sd ~ sd'), s('hh*8'))",
  description: "Drums",
  wait_cycles: 0
})

// Call 2: Harmonic layer
apply_live_code_edit({
  code: "stack(...drums, note('c2').s('saw').lpf(400), note('c4 e4 g4').s('tri'))",
  description: "Bass + Melody",
  wait_cycles: 8
})
```

### ❌ Bad Chunking (Too many calls)

```javascript
// DON'T DO THIS - too fragmented
❌ Call 1: Just kick
❌ Call 2: Just snare
❌ Call 3: Just hats
❌ Call 4: Just bass
❌ Call 5: Just melody
```

### Parameter Guidelines

- **mode**: Use `"replace"` for most cases
- **description**: Brief, clear (shown in UI)
- **wait_cycles**:
  - `0` for first change (apply immediately)
  - `4-8` for build-up sections
  - `8-16` for major transitions

---

## System Prompt Instructions

The agent's system prompt includes:

```
## Queue Mode (Progressive Building)

When Queue Mode (🎬) is enabled, use apply_live_code_edit with
description and wait_cycles parameters.

**Best practice: 2-4 substantial musical changes work well**
Think in musical sections rather than individual instruments.
Combine elements that belong together.

**Parameters:**
- description: Brief label ("Drums", "Add bass layer")
- wait_cycles: 0 for first, 8-16 for subsequent
- Combine related elements in one call
```

---

## Testing

### Manual Test

1. Build desktop app: `pnpm tauri:dev`
2. Open Chat tab
3. Click 🎬 to enable Queue Mode
4. Ask: "Build a techno beat progressively"
5. Observe:
   - Agent makes multiple tool calls
   - Changes appear in queue UI
   - Auto-applying as cycles elapse
   - Manual controls work (Skip, Apply All, etc.)

### Test Cases

- ✅ Queue toggle button works
- ✅ Tool calls populate queue
- ✅ Changes apply in order
- ✅ Auto-advance respects wait_cycles
- ✅ Manual controls work (Apply Next, Skip, etc.)
- ✅ Cycle tracking counts accurately
- ✅ Preview shows code correctly
- ✅ Clear removes all items
- ✅ Works with tempo changes

---

## Files Modified

### Backend
- `src-tauri/src/tools.rs` - Tool definition with queue params
- `src-tauri/src/chatbridge.rs` - System prompt instructions

### Frontend
- `website/src/repl/useReplContext.jsx` - State management & auto-advance
- `website/src/repl/components/panel/ChatTab.jsx` - UI & event listener

---

## Future Enhancements

- [ ] Undo/redo for applied changes
- [ ] Save queue sequences as presets
- [ ] Visual timeline of queued changes
- [ ] Branching queues (alternate paths)
- [ ] Queue templates for common workflows
- [ ] Metronome/click track during queue

---

## Troubleshooting

### Changes Not Auto-Applying?
- Check that pattern is playing (not paused)
- Verify Queue Mode is enabled (🎬 button)
- Look for cycle counter incrementing

### Queue Not Populating?
- Check browser console for events
- Verify agent is calling tool with `description` param
- Check Rust logs for tool execution

### Timing Issues?
- Tempo changes affect cycle counting (this is correct)
- Pattern must be playing for cycles to increment
- Manual controls always work regardless of timing

---

## Learn More

- [Chat Assistant Setup](./chat-assistant.md)
- [RAG Semantic Search](./rag-search.md)
- [Strudel Documentation](https://strudel.cc)
