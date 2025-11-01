//! Mini notation parser and evaluator for Strudel
//!
//! This crate provides parsing, formatting, and evaluation of Strudel's mini notation,
//! a concise syntax for expressing rhythmic patterns.
//!
//! # Examples
//!
//! ```
//! use strudel_mini::{parse, evaluate};
//!
//! // Parse mini notation
//! let ast = parse("bd sd [cp cp] ~").unwrap();
//!
//! // Evaluate to a pattern
//! let pattern = evaluate(&ast).unwrap();
//! ```
//!
//! # Mini Notation Syntax
//!
//! - Space-separated sequences: `a b c`
//! - Nested groups: `[a b]`
//! - Stacking (layering): `a,b,c`
//! - Polymeter: `{a b c, d e}`
//! - Random choice: `a|b|c`
//! - Silence: `~`
//! - Replication: `a!3`
//! - Euclidean rhythms: `bd(3,8)`
//!
//! # Main Functions
//!
//! - [`parse`]: Parse mini notation string to AST
//! - [`evaluate`]: Evaluate AST to executable pattern
//! - [`format()`]: Format AST back to mini notation
//! - [`extract_patterns`]: Extract mini notation from .strudel files

pub mod ast;
pub mod error;
pub mod evaluator;
pub mod formatter;
pub mod lexer;
pub mod parser;
pub mod preprocessor;
pub mod span;

#[cfg(test)]
mod parser_tests;

pub use ast::{Ast, Alignment};
pub use error::{ParseError, Result};
pub use evaluator::evaluate;
pub use formatter::format;
pub use lexer::{Lexer, Token};
pub use parser::{parse, parse_mini, Parser};
pub use preprocessor::{extract_patterns, combine_patterns, CombineStrategy, ExtractedPattern};
pub use span::Span;
