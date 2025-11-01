# strudel-core

Core types and pattern combinators for the Strudel pattern language in Rust.

## Overview

`strudel-core` provides the foundational types and functions for working with Strudel-style patterns in Rust. It includes pattern combinators, precise timing utilities, and value types that form the basis of the Strudel ecosystem.

## Features

- **Pattern System**: Functional pattern representation and querying
- **Precise Timing**: Rational number-based timing (no floating point errors)
- **Pattern Combinators**: Functions for combining and transforming patterns
- **Euclidean Rhythms**: Bjorklund algorithm implementation
- **Value Types**: Support for strings, numbers, and structured values

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
strudel-core = "0.1.0"
```

## Quick Start

```rust
use strudel-core::{pure, sequence, stack, fastcat, Value, State, TimeSpan, Fraction};

// Create a simple drum pattern
let pattern = sequence(vec![
    pure(Value::String("bd".into())),
    pure(Value::String("sd".into())),
    pure(Value::String("hh".into())),
    pure(Value::String("cp".into())),
]);

// Query the pattern for events in the first cycle
let span = TimeSpan::new(Fraction::from(0), Fraction::from(1));
let state = State::new(span);
let events = pattern.query(state);

// Stack multiple patterns (play simultaneously)
let kick = pure(Value::String("bd".into()));
let snare = pure(Value::String("sd".into()));
let layered = stack(vec![kick, snare]);
```

## Core Concepts

### Pattern

A `Pattern` represents a function from time to events. You can query a pattern over any time span to get the events that occur during that time.

```rust
use strudel_core::{Pattern, State, TimeSpan, Fraction};

fn query_pattern(pattern: Pattern) {
    let span = TimeSpan::new(Fraction::from(0), Fraction::from(1));
    let state = State::new(span);
    let haps = pattern.query(state);

    for hap in haps {
        println!("Event: {:?} at {:?}", hap.value, hap.whole);
    }
}
```

### Combinators

Combinators are functions that create or transform patterns:

- `pure(value)` - Create a pattern with a single value
- `silence()` - Create an empty pattern
- `sequence(patterns)` - Play patterns one after another (fastcat)
- `stack(patterns)` - Play patterns simultaneously
- `slowcat(patterns)` - Like sequence but slower
- `polymeter(patterns)` - Polymetric patterns
- `choose(patterns, seed)` - Random choice
- `choose_weighted(patterns, seed)` - Weighted random choice

```rust
use strudel_core::{sequence, stack, pure, Value};

// Sequence: play one after another
let seq = sequence(vec![
    pure(Value::String("a".into())),
    pure(Value::String("b".into())),
    pure(Value::String("c".into())),
]);

// Stack: play simultaneously
let stacked = stack(vec![
    pure(Value::String("bd".into())),
    pure(Value::String("hh".into())),
]);
```

### Euclidean Rhythms

Generate Euclidean rhythms using the Bjorklund algorithm:

```rust
use strudel_core::bjorklund;

// 3 pulses distributed over 8 steps
let pattern = bjorklund(3, 8, 0);
// Returns: [true, false, false, true, false, false, true, false]

// With rotation
let rotated = bjorklund(3, 8, 2);
```

### Time and Values

```rust
use strudel_core::{Fraction, TimeSpan, Value};

// Precise timing with fractions
let quarter = Fraction::new(1, 4);
let half = Fraction::new(1, 2);

// Time spans
let span = TimeSpan::new(Fraction::from(0), Fraction::from(1));

// Values can be strings, numbers, or structured
let string_val = Value::String("bd".into());
let number_val = Value::Number(440.0);
let list_val = Value::List(vec![
    Value::String("note".into()),
    Value::Number(60.0),
]);
```

## Pattern Methods

Patterns support various transformation methods:

```rust
use strudel_core::{pure, Value};

let pattern = pure(Value::String("bd".into()));

// Time manipulation
let faster = pattern.fast(2.0);      // Speed up by 2x
let slower = pattern.slow(2.0);      // Slow down by 2x
let early = pattern.early(0.25);     // Shift earlier
let late = pattern.late(0.25);       // Shift later

// Structural
let repeated = pattern.struct_(pure(Value::Number(3.0))); // Apply structure
```

## Example: Creating a Drum Pattern

```rust
use strudel_core::{sequence, stack, pure, Value};

fn create_drum_pattern() -> strudel_core::Pattern {
    // Kick drum on 1 and 3
    let kick = sequence(vec![
        pure(Value::String("bd".into())),
        pure(Value::String("~".into())),
        pure(Value::String("bd".into())),
        pure(Value::String("~".into())),
    ]);

    // Snare on 2 and 4
    let snare = sequence(vec![
        pure(Value::String("~".into())),
        pure(Value::String("sd".into())),
        pure(Value::String("~".into())),
        pure(Value::String("sd".into())),
    ]);

    // Hi-hat on every beat
    let hihat = sequence(vec![
        pure(Value::String("hh".into())),
        pure(Value::String("hh".into())),
        pure(Value::String("hh".into())),
        pure(Value::String("hh".into())),
    ]);

    // Layer them together
    stack(vec![kick, snare, hihat])
}
```

## License

AGPL-3.0-or-later

## Links

- [Repository](https://github.com/Emanuel-de-Jong/MIDI-To-Strudel)
- [Strudel Official Site](https://strudel.cc/)
