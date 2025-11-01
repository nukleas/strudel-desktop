// Formatter: Convert AST back to mini notation
use crate::ast::*;

/// Format an AST node as mini notation string
pub fn format(ast: &Ast) -> String {
    match ast {
        Ast::Atom(atom) => format_atom(atom),
        Ast::Pattern(pattern) => format_pattern(pattern),
        Ast::Element(element) => format_element(element),
        Ast::Operator(op) => format_operator(op),
        Ast::Command(cmd) => format_command(cmd),
    }
}

fn format_atom(atom: &AtomNode) -> String {
    match &atom.value {
        AtomValue::Number(n) => {
            // Format number nicely
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        AtomValue::String(s) => s.clone(),
        AtomValue::Silence => "~".to_string(),
    }
}

fn format_pattern(pattern: &PatternNode) -> String {
    let children: Vec<String> = pattern.children.iter().map(format).collect();

    match pattern.alignment {
        Alignment::Fastcat => {
            // Space-separated sequence
            children.join(" ")
        }
        Alignment::Stack => {
            // Comma-separated stack
            children.join(",")
        }
        Alignment::Rand => {
            // Pipe-separated random choice
            children.join(" | ")
        }
        Alignment::Polymeter => {
            // Curly braces polymeter
            let inner = children.join(", ");
            if let Some(steps) = &pattern.steps_per_cycle {
                format!("{{{}}}%{}", inner, format(steps))
            } else {
                format!("{{{}}}", inner)
            }
        }
        Alignment::PolymeterSlowcat => {
            // Angle brackets slow sequence
            format!("<{}>", children.join(" "))
        }
        Alignment::Feet => {
            // Dot-separated feet
            children.join(" . ")
        }
    }
}

fn format_element(element: &ElementNode) -> String {
    let mut result = String::new();

    // Check if source needs wrapping
    match element.source.as_ref() {
        Ast::Pattern(pattern) => {
            // Patterns used as elements need to be wrapped in delimiters
            match pattern.alignment {
                Alignment::Polymeter => {
                    // Already has braces in format_pattern
                    result.push_str(&format_pattern(pattern));
                }
                Alignment::PolymeterSlowcat => {
                    // Already has angle brackets in format_pattern
                    result.push_str(&format_pattern(pattern));
                }
                _ => {
                    // All other patterns (Fastcat, Stack, Rand, Feet) need brackets
                    result.push('[');
                    result.push_str(&format_pattern(pattern));
                    result.push(']');
                }
            }
        }
        _ => {
            result.push_str(&format(&element.source));
        }
    }

    // Add operators
    for op in &element.ops {
        result.push_str(&format_slice_op(op));
    }

    // Add weight if not default (1.0)
    if element.weight != 1.0 && element.weight.fract() == 0.0 {
        let weight = element.weight as i64 - 1;
        if weight > 0 {
            result.push_str(&"_".repeat(weight as usize));
        }
    }

    result
}

fn format_slice_op(op: &SliceOp) -> String {
    match op {
        SliceOp::Stretch { amount, op_type } => {
            let op_char = match op_type {
                StretchType::Fast => "*",
                StretchType::Slow => "/",
            };
            format!("{}{}", op_char, format(amount))
        }
        SliceOp::Replicate { amount } => {
            format!("!{}", amount)
        }
        SliceOp::Bjorklund { pulse, step, rotation } => {
            if let Some(rot) = rotation {
                format!("({},{},{})", format(pulse), format(step), format(rot))
            } else {
                format!("({},{})", format(pulse), format(step))
            }
        }
        SliceOp::DegradeBy { amount, seed: _ } => {
            if let Some(amt) = amount {
                format!("?{}", amt)
            } else {
                "?".to_string()
            }
        }
        SliceOp::Tail { element } => {
            format!(":{}", format(element))
        }
        SliceOp::Range { element } => {
            format!("..{}", format(element))
        }
    }
}

fn format_operator(op: &OperatorNode) -> String {
    let op_name = match op.op_type {
        OperatorType::Slow => "slow",
        OperatorType::Fast => "fast",
        OperatorType::Scale => "scale",
        OperatorType::Struct => "struct",
        OperatorType::Shift => "shift",
        OperatorType::Bjorklund => "bjorklund",
        OperatorType::Target => "target",
    };

    let args = match &op.args {
        OperatorArgs::Number(n) => n.to_string(),
        OperatorArgs::String(s) => format!("\"{}\"", s),
        OperatorArgs::Pattern(p) => format(p),
        OperatorArgs::Bjorklund { pulse, step, rotation } => {
            if let Some(rot) = rotation {
                format!("{},{},{}", pulse, step, rot)
            } else {
                format!("{},{}", pulse, step)
            }
        }
    };

    format!("{}({}) {}", op_name, args, format(&op.source))
}

fn format_command(cmd: &CommandNode) -> String {
    let cmd_name = match cmd.cmd_type {
        CommandType::Setcps => "setcps",
        CommandType::Setbpm => "setbpm",
        CommandType::Hush => "hush",
    };

    if let Some(val) = cmd.value {
        format!("{} {}", cmd_name, val)
    } else {
        cmd_name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn roundtrip(input: &str) {
        let ast = parse(input).unwrap();
        let formatted = format(&ast);
        let ast2 = parse(&formatted).unwrap();

        // Both ASTs should be equivalent (ignoring spans)
        // For now, just check it parses
        assert_eq!(
            serde_json::to_value(&ast).unwrap(),
            serde_json::to_value(&ast2).unwrap(),
            "Roundtrip failed for '{}' -> '{}' -> AST", input, formatted
        );
    }

    #[test]
    fn test_format_atom() {
        roundtrip("bd");
        roundtrip("42");
        roundtrip("3.14");
        roundtrip("~");
    }

    #[test]
    fn test_format_sequence() {
        roundtrip("bd sd cp hh");
    }

    #[test]
    fn test_format_fast() {
        roundtrip("bd*2");
        roundtrip("sd*4");
    }

    #[test]
    fn test_format_slow() {
        roundtrip("bd/2");
        roundtrip("cp/3");
    }

    #[test]
    fn test_format_brackets() {
        roundtrip("bd [sd cp]");
    }

    #[test]
    fn test_format_stack() {
        roundtrip("bd,sd,cp");
    }

    #[test]
    fn test_format_polymeter() {
        roundtrip("{bd sd, cp hh oh}");
    }

    #[test]
    fn test_format_slow_sequence() {
        roundtrip("<bd sd cp>");
    }

    #[test]
    fn test_format_euclidean() {
        roundtrip("bd(3,8)");
        roundtrip("sd(5,16,2)");
    }

    #[test]
    fn test_format_complex() {
        roundtrip("bd*2 [sd cp]*3");
    }
}
