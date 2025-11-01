# Rust Pattern Commands

The Strudel desktop app now includes Rust-powered pattern manipulation commands through the `strudelbridge` module. These commands provide faster, type-safe pattern parsing, validation, formatting, and analysis.

## Available Commands

### 1. `validate_pattern`

Validates a mini notation pattern and returns detailed error information if invalid.

**TypeScript Usage:**
```typescript
import { invoke } from '@tauri-apps/api/core';

// Validate a pattern
try {
  await invoke('validate_pattern', { pattern: 'bd sd cp' });
  console.log('Pattern is valid!');
} catch (error) {
  // Error object contains { message, location }
  console.error('Validation error:', error);
  // location: { line, column, span_start, span_end }
}
```

**Returns:**
- `Ok(())` if valid
- `Err(StrudelError)` if invalid, with error location

### 2. `format_pattern`

Formats a mini notation pattern to canonical form.

**TypeScript Usage:**
```typescript
import { invoke } from '@tauri-apps/api/core';

// Format a pattern
const formatted = await invoke('format_pattern', {
  pattern: 'bd   sd    cp'
});
console.log(formatted); // "bd sd cp" (normalized)
```

**Returns:**
- Formatted string

### 3. `evaluate_pattern`

Evaluates a pattern and returns the events (Haps) that occur in a given time range.

**TypeScript Usage:**
```typescript
import { invoke } from '@tauri-apps/api/core';

// Get events for first cycle
const events = await invoke('evaluate_pattern', {
  pattern: 'bd sd [cp cp]',
  fromCycle: 0.0,
  durationCycles: 1.0
});

// events is an array of:
// {
//   value: { String: "bd" } | { Number: 42 } | ...
//   part_begin: 0.0,
//   part_end: 0.333,
//   whole_begin: 0.0,
//   whole_end: 0.333
// }

console.log(`Found ${events.length} events`);
events.forEach(event => {
  console.log(`${event.value} at ${event.part_begin}-${event.part_end}`);
});
```

**Returns:**
- Array of `SerializableHap` objects

### 4. `analyze_pattern`

Analyzes a pattern and returns metrics like event count, density, and unique values.

**TypeScript Usage:**
```typescript
import { invoke } from '@tauri-apps/api/core';

// Analyze a pattern over 2 cycles
const metrics = await invoke('analyze_pattern', {
  pattern: 'bd sd [cp hh]',
  cycles: 2.0
});

console.log(metrics);
// {
//   event_count: 8,
//   density: 4.0,  // events per cycle
//   cycles: 2.0,
//   unique_values: ["bd", "sd", "cp", "hh"]
// }
```

**Returns:**
- `PatternMetrics` object with:
  - `event_count`: Total number of events
  - `density`: Events per cycle
  - `cycles`: Number of cycles analyzed
  - `unique_values`: Array of unique value strings

## Error Handling

All commands return errors with this structure:

```typescript
interface StrudelError {
  message: string;
  location?: {
    line: number;
    column: number;
    span_start: number;
    span_end: number;
  };
}
```

## Example: Real-time Validation in Editor

```typescript
import { invoke } from '@tauri-apps/api/core';
import { debounce } from 'lodash';

// In your CodeMirror onChange handler
const validatePattern = debounce(async (code: string) => {
  try {
    await invoke('validate_pattern', { pattern: code });
    // Clear error markers
    clearErrors();
  } catch (error) {
    // Show error at location
    if (error.location) {
      showErrorAtPosition(
        error.location.span_start,
        error.location.span_end,
        error.message
      );
    }
  }
}, 300);

editor.on('change', (instance) => {
  validatePattern(instance.getValue());
});
```

## Example: Pattern Visualization

```typescript
import { invoke } from '@tauri-apps/api/core';

async function visualizePattern(pattern: string) {
  try {
    const events = await invoke('evaluate_pattern', {
      pattern,
      fromCycle: 0.0,
      durationCycles: 4.0
    });

    // Render timeline
    events.forEach(event => {
      const x = event.part_begin * timelineWidth;
      const width = (event.part_end - event.part_begin) * timelineWidth;

      drawEventBox(x, width, event.value);
    });
  } catch (error) {
    console.error('Failed to visualize:', error);
  }
}
```

## Performance Notes

- Pattern parsing and evaluation is done in Rust, which is significantly faster than JavaScript
- Validation is near-instant even for complex patterns
- Consider debouncing real-time validation to avoid unnecessary invocations
- The Rust implementation uses exact fractional arithmetic, avoiding floating-point errors

## Next Steps

Future enhancements could include:
- Pattern transformation commands (reverse, transpose, etc.)
- MIDI file import/export
- Native audio rendering
- Pattern similarity/diff analysis
- Advanced pattern analysis (polyrhythm detection, complexity metrics)
