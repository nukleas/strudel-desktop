use crate::span::Span;
use serde::{Deserialize, Serialize};

/// Abstract Syntax Tree for mini notation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Ast {
    Atom(AtomNode),
    Pattern(PatternNode),
    Element(ElementNode),
    Operator(OperatorNode),
    Command(CommandNode),
}

impl Ast {
    pub fn span(&self) -> Span {
        match self {
            Ast::Atom(node) => node.span,
            Ast::Pattern(node) => node.span,
            Ast::Element(node) => node.span,
            Ast::Operator(node) => node.span,
            Ast::Command(node) => node.span,
        }
    }
}

/// Atom - a leaf value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtomNode {
    pub value: AtomValue,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AtomValue {
    Number(f64),
    String(String),
    Silence, // ~ or -
}

impl AtomNode {
    pub fn new(value: AtomValue, span: Span) -> Self {
        AtomNode { value, span }
    }

    pub fn number(n: f64, span: Span) -> Self {
        AtomNode::new(AtomValue::Number(n), span)
    }

    pub fn string(s: impl Into<String>, span: Span) -> Self {
        AtomNode::new(AtomValue::String(s.into()), span)
    }

    pub fn silence(span: Span) -> Self {
        AtomNode::new(AtomValue::Silence, span)
    }
}

/// Pattern - a composite pattern
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternNode {
    pub children: Vec<Ast>,
    pub alignment: Alignment,
    pub seed: Option<u64>,
    pub steps_marked: bool,
    pub steps_per_cycle: Option<Box<Ast>>,
    pub span: Span,
}

impl PatternNode {
    pub fn new(
        children: Vec<Ast>,
        alignment: Alignment,
        seed: Option<u64>,
        steps_marked: bool,
        span: Span,
    ) -> Self {
        PatternNode {
            children,
            alignment,
            seed,
            steps_marked,
            steps_per_cycle: None,
            span,
        }
    }

    pub fn with_steps_per_cycle(mut self, steps: Ast) -> Self {
        self.steps_per_cycle = Some(Box::new(steps));
        self
    }
}

/// Alignment type for patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignment {
    Fastcat,          // sequence (space-separated)
    Stack,            // layering (comma-separated)
    Rand,             // random choice (pipe-separated)
    Polymeter,        // polymeter ({})
    PolymeterSlowcat, // slow sequence (<>)
    Feet,             // dot operator (.)
}

/// Element - a slice with modifiers
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementNode {
    pub source: Box<Ast>,
    pub ops: Vec<SliceOp>,
    pub weight: f64,
    pub reps: usize,
    pub span: Span,
}

impl ElementNode {
    pub fn new(source: Ast, span: Span) -> Self {
        ElementNode {
            source: Box::new(source),
            ops: Vec::new(),
            weight: 1.0,
            reps: 1,
            span,
        }
    }

    pub fn with_ops(mut self, ops: Vec<SliceOp>) -> Self {
        self.ops = ops;
        self
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_reps(mut self, reps: usize) -> Self {
        self.reps = reps;
        self
    }

    pub fn add_op(&mut self, op: SliceOp) {
        self.ops.push(op);
    }
}

/// Slice operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SliceOp {
    Stretch {
        amount: Box<Ast>,
        op_type: StretchType,
    },
    Replicate {
        amount: usize,
    },
    Bjorklund {
        pulse: Box<Ast>,
        step: Box<Ast>,
        rotation: Option<Box<Ast>>,
    },
    DegradeBy {
        amount: Option<f64>,
        seed: u64,
    },
    Tail {
        element: Box<Ast>,
    },
    Range {
        element: Box<Ast>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StretchType {
    Fast, // *
    Slow, // /
}

/// Operator - Haskell-style operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorNode {
    pub op_type: OperatorType,
    pub args: OperatorArgs,
    pub source: Box<Ast>,
    pub span: Span,
}

impl OperatorNode {
    pub fn new(op_type: OperatorType, args: OperatorArgs, source: Ast, span: Span) -> Self {
        OperatorNode {
            op_type,
            args,
            source: Box::new(source),
            span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatorType {
    Slow,
    Fast,
    Scale,
    Struct,
    Shift,
    Bjorklund,
    Target,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperatorArgs {
    Number(f64),
    String(String),
    Pattern(Box<Ast>),
    Bjorklund {
        pulse: i64,
        step: i64,
        rotation: Option<i64>,
    },
}

/// Command - control commands
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandNode {
    pub cmd_type: CommandType,
    pub value: Option<f64>,
    pub span: Span,
}

impl CommandNode {
    pub fn new(cmd_type: CommandType, value: Option<f64>, span: Span) -> Self {
        CommandNode {
            cmd_type,
            value,
            span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandType {
    Setcps,
    Setbpm,
    Hush,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atom_creation() {
        let atom = AtomNode::string("bd", Span::new(0, 2));
        assert_eq!(atom.value, AtomValue::String("bd".to_string()));
        assert_eq!(atom.span, Span::new(0, 2));
    }

    #[test]
    fn test_pattern_creation() {
        let atom1 = Ast::Atom(AtomNode::string("bd", Span::new(0, 2)));
        let atom2 = Ast::Atom(AtomNode::string("sd", Span::new(3, 5)));
        let pattern = PatternNode::new(
            vec![atom1, atom2],
            Alignment::Fastcat,
            None,
            false,
            Span::new(0, 5),
        );
        assert_eq!(pattern.children.len(), 2);
        assert_eq!(pattern.alignment, Alignment::Fastcat);
    }

    #[test]
    fn test_element_creation() {
        let atom = Ast::Atom(AtomNode::string("bd", Span::new(0, 2)));
        let element = ElementNode::new(atom, Span::new(0, 2));
        assert_eq!(element.weight, 1.0);
        assert_eq!(element.reps, 1);
        assert_eq!(element.ops.len(), 0);
    }
}
