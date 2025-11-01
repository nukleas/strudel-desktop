use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a value in a Strudel pattern
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// Number value
    Number(f64),
    /// String value (note names, sample names, etc.)
    String(String),
    /// Boolean value
    Bool(bool),
    /// List of values
    List(Vec<Value>),
    /// Silence/rest
    Silence,
}

impl Value {
    /// Check if this value is silence
    pub fn is_silence(&self) -> bool {
        matches!(self, Value::Silence)
    }

    /// Try to extract a number
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to extract a string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to extract a list
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(list) => Some(list),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Silence => write!(f, "~"),
        }
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Number(n)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n as f64)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<Vec<Value>> for Value {
    fn from(list: Vec<Value>) -> Self {
        Value::List(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_creation() {
        let v1 = Value::from(42.0);
        assert_eq!(v1.as_number(), Some(42.0));

        let v2 = Value::from("bd");
        assert_eq!(v2.as_string(), Some("bd"));

        let v3 = Value::Silence;
        assert!(v3.is_silence());
    }

    #[test]
    fn test_value_display() {
        assert_eq!(Value::from(42.0).to_string(), "42");
        assert_eq!(Value::from("bd").to_string(), "bd");
        assert_eq!(Value::Silence.to_string(), "~");
    }
}
