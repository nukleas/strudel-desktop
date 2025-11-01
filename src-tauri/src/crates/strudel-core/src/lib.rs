//! Core types and utilities for Strudel pattern language
//!
//! This crate provides the foundational types and functions for working with
//! Strudel-style patterns in Rust. It includes pattern combinators, timing
//! utilities, and value types that form the basis of the Strudel ecosystem.
//!
//! # Examples
//!
//! ```
//! use strudel_core::{pure, sequence, fastcat, Value};
//!
//! // Create a simple pattern
//! let pattern = sequence(vec![
//!     pure(Value::String("bd".into())),
//!     pure(Value::String("sd".into())),
//! ]);
//! ```
//!
//! # Main Components
//!
//! - **Pattern**: The core pattern type
//! - **Value**: Values that patterns can contain (strings, numbers, etc.)
//! - **Hap**: A pattern event with timing and value
//! - **TimeSpan**: Represents time intervals
//! - **Combinators**: Functions for combining and transforming patterns

pub mod combinators;
pub mod euclid;
pub mod fraction;
pub mod hap;
pub mod pattern;
pub mod state;
pub mod timespan;
pub mod value;

pub use combinators::{choose, choose_weighted, fastcat, polymeter, polyrhythm, pure, sequence, silence, slowcat, stack};
pub use euclid::bjorklund;
pub use fraction::Fraction;
pub use hap::{Context, Hap};
pub use pattern::Pattern;
pub use state::State;
pub use timespan::TimeSpan;
pub use value::Value;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
