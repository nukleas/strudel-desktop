use crate::span::Span;
use std::fmt;

pub type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedToken {
        expected: String,
        found: String,
        span: Span,
    },
    UnexpectedEof {
        expected: String,
    },
    UnclosedDelimiter {
        delimiter: char,
        open_span: Span,
    },
    InvalidNumber {
        value: String,
        span: Span,
    },
    InvalidAtom {
        value: String,
        span: Span,
    },
    Custom {
        message: String,
        span: Option<Span>,
    },
}

impl ParseError {
    pub fn unexpected_token(expected: impl Into<String>, found: impl Into<String>, span: Span) -> Self {
        ParseError::UnexpectedToken {
            expected: expected.into(),
            found: found.into(),
            span,
        }
    }

    pub fn unexpected_eof(expected: impl Into<String>) -> Self {
        ParseError::UnexpectedEof {
            expected: expected.into(),
        }
    }

    pub fn unclosed_delimiter(delimiter: char, open_span: Span) -> Self {
        ParseError::UnclosedDelimiter { delimiter, open_span }
    }

    pub fn invalid_number(value: impl Into<String>, span: Span) -> Self {
        ParseError::InvalidNumber {
            value: value.into(),
            span,
        }
    }

    pub fn invalid_atom(value: impl Into<String>, span: Span) -> Self {
        ParseError::InvalidAtom {
            value: value.into(),
            span,
        }
    }

    pub fn custom(message: impl Into<String>, span: Option<Span>) -> Self {
        ParseError::Custom {
            message: message.into(),
            span,
        }
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            ParseError::UnexpectedToken { span, .. } => Some(*span),
            ParseError::UnexpectedEof { .. } => None,
            ParseError::UnclosedDelimiter { open_span, .. } => Some(*open_span),
            ParseError::InvalidNumber { span, .. } => Some(*span),
            ParseError::InvalidAtom { span, .. } => Some(*span),
            ParseError::Custom { span, .. } => *span,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, found, span } => {
                write!(f, "Expected {}, found {} at {}", expected, found, span)
            }
            ParseError::UnexpectedEof { expected } => {
                write!(f, "Unexpected end of input, expected {}", expected)
            }
            ParseError::UnclosedDelimiter { delimiter, open_span } => {
                write!(f, "Unclosed delimiter '{}' opened at {}", delimiter, open_span)
            }
            ParseError::InvalidNumber { value, span } => {
                write!(f, "Invalid number '{}' at {}", value, span)
            }
            ParseError::InvalidAtom { value, span } => {
                write!(f, "Invalid atom '{}' at {}", value, span)
            }
            ParseError::Custom { message, span } => {
                if let Some(span) = span {
                    write!(f, "{} at {}", message, span)
                } else {
                    write!(f, "{}", message)
                }
            }
        }
    }
}

impl std::error::Error for ParseError {}
