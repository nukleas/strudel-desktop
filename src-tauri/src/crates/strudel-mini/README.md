# strudel-mini

Mini notation parser and evaluator for Strudel patterns.

## Overview

`strudel-mini` provides parsing, formatting, and evaluation of Strudel's mini notation - a concise syntax for expressing rhythmic patterns. It can be used as a library or as a command-line tool.

## Features

- **Complete Mini Notation Parser**: Supports all Strudel mini notation syntax
- **AST Evaluation**: Convert parsed patterns to executable `strudel-core` patterns
- **Pattern Extraction**: Extract mini notation from `.strudel` JavaScript files
- **Formatter**: Pretty-print parsed patterns back to mini notation
- **CLI Tool**: Validate, parse, evaluate, and play patterns from the command line

## Installation

### As a Library

```toml
[dependencies]
strudel-mini = "0.1.0"
```

### As a CLI Tool

```bash
cargo install strudel-mini
```

## Mini Notation Syntax

### Basic Elements

```
a b c         # Space-separated sequence (fastcat)
[a b c]       # Grouped sub-pattern
~             # Silence/rest
a!3           # Replication (a a a)
```

### Operators

```
a,b,c         # Stack (play simultaneously)
a|b|c         # Random choice
a.b.c         # Feet/dot operator
{a b, c d e}  # Polymeter
<a b c>       # Polymeter slowcat
```

### Euclidean Rhythms

```
bd(3,8)       # 3 pulses over 8 steps
bd(3,8,2)     # With rotation of 2
```

### Ranges

```
0..7          # Ascending range [0,1,2,3,4,5,6,7]
7..0          # Descending range [7,6,5,4,3,2,1,0]
```

### Elongation

```
a@2 b c       # 'a' takes twice as long
```

### Weights

```
a|b%2|c       # 'b' is twice as likely to be chosen
```

## Quick Start (Library)

```rust
use strudel_mini::{parse, evaluate, format};
use strudel_core::{State, TimeSpan, Fraction};

// Parse mini notation
let ast = parse("bd sd [cp cp] ~").unwrap();

// Evaluate to a pattern
let pattern = evaluate(&ast).unwrap();

// Query the pattern
let span = TimeSpan::new(Fraction::from(0), Fraction::from(1));
let state = State::new(span);
let events = pattern.query(state);

for event in events {
    println!("{:?}", event.value);
}

// Format back to mini notation
let formatted = format(&ast);
println!("{}", formatted);
```

## CLI Usage

### Validate a Pattern

```bash
strudel-mini validate "bd sd hh cp"
```

### Parse and Show AST

```bash
strudel-mini ast "bd [sd cp]" --output-format json
```

### Evaluate and Show Events

```bash
strudel-mini eval "bd sd hh cp" --from 0 --duration 1
```

### Format a Pattern

```bash
strudel-mini fmt "bd   sd    [  cp  cp  ]"
# Output: bd sd [cp cp]
```

### Extract from .strudel Files

```bash
# Extract and show separately
strudel-mini extract myfile.strudel --strategy separate

# Extract and stack all patterns
strudel-mini extract myfile.strudel --strategy stack

# Extract and sequence all patterns
strudel-mini extract myfile.strudel --strategy sequence
```

### Play Patterns (with audio feature)

```bash
# Play a simple pattern
strudel-mini play "bd sd hh cp" --tempo 120 --duration 10

# Play from a file
strudel-mini play --file pattern.txt --tempo 140

# Play from a .strudel file
strudel-mini play --strudel-file song.strudel --combine stack
```

## Examples

### Simple Drum Pattern

```rust
use strudel_mini::{parse, evaluate};

let pattern_str = "bd sd hh cp";
let ast = parse(pattern_str).unwrap();
let pattern = evaluate(&ast).unwrap();
```

### Nested Groups

```rust
let pattern_str = "[[bd sd] [hh cp]] [[bd bd] ~]";
let ast = parse(pattern_str).unwrap();
let pattern = evaluate(&ast).unwrap();
```

### Stacking (Layering)

```rust
// Kick and snare playing simultaneously
let pattern_str = "bd,sd";
let ast = parse(pattern_str).unwrap();
let pattern = evaluate(&ast).unwrap();
```

### Random Choice

```rust
// Randomly choose between bd, sd, and cp
let pattern_str = "bd|sd|cp";
let ast = parse(pattern_str).unwrap();
let pattern = evaluate(&ast).unwrap();
```

### Euclidean Rhythm

```rust
// 3 hits over 8 steps
let pattern_str = "bd(3,8)";
let ast = parse(pattern_str).unwrap();
let pattern = evaluate(&ast).unwrap();
```

### Polymeter

```rust
// Two patterns with different cycle lengths
let pattern_str = "{bd sd hh, cp cp}";
let ast = parse(pattern_str).unwrap();
let pattern = evaluate(&ast).unwrap();
```

### Complex Example

```rust
let pattern_str = r#"
    [bd sd cp hh]
    [bd [sd cp]]
    [bd!2 [sd cp]!2]
    ~
    [bd|sd|cp]
    bd(3,8)
"#;
let ast = parse(pattern_str).unwrap();
let pattern = evaluate(&ast).unwrap();
```

## Extracting from .strudel Files

The preprocessor can extract mini notation patterns from Strudel JavaScript files:

```rust
use strudel_mini::{extract_patterns, combine_patterns, CombineStrategy};

let source = r#"
setcpm(120/4)

$: note(`bd sd hh cp`).sound()
$: note(`c3 e3 g3`).sound("piano")
"#;

let patterns = extract_patterns(source);
let combined = combine_patterns(&patterns, CombineStrategy::Stack);
println!("{}", combined);
```

## Error Handling

```rust
use strudel_mini::{parse, ParseError};

match parse("bd [ sd") {
    Ok(ast) => println!("Parsed: {:?}", ast),
    Err(ParseError::UnexpectedEof { expected, span }) => {
        eprintln!("Unexpected end of input, expected: {}", expected);
    }
    Err(e) => eprintln!("Parse error: {}", e),
}
```

## API Documentation

### Main Functions

- `parse(input: &str) -> Result<Ast>` - Parse mini notation to AST
- `evaluate(ast: &Ast) -> Result<Pattern>` - Evaluate AST to pattern
- `format(ast: &Ast) -> String` - Format AST to mini notation
- `extract_patterns(source: &str) -> Vec<ExtractedPattern>` - Extract from .strudel files
- `combine_patterns(patterns: &[ExtractedPattern], strategy: CombineStrategy) -> String` - Combine extracted patterns

### Types

- `Ast` - Abstract syntax tree node
- `ParseError` - Parse error with span information
- `Alignment` - Pattern alignment (Fastcat, Stack, Rand, etc.)
- `CombineStrategy` - Strategy for combining patterns (Stack, Sequence, First, Separate)

## Feature Flags

- `audio` - Enable audio playback support (requires `strudel-audio`)

```toml
[dependencies]
strudel-mini = { version = "0.1.0", features = ["audio"] }
```

## License

AGPL-3.0-or-later

## Links

- [Repository](https://github.com/Emanuel-de-Jong/MIDI-To-Strudel)
- [Strudel Official Site](https://strudel.cc/)
- [Mini Notation Reference](https://strudel.cc/learn/mini-notation)
