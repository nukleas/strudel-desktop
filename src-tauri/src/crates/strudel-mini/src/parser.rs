use crate::ast::*;
use crate::error::{ParseError, Result};
use crate::lexer::{Lexer, Token};
use crate::span::Span;

/// Parser for mini notation
pub struct Parser<'source> {
    lexer: Lexer<'source>,
    seed_counter: u64,
}

impl<'source> Parser<'source> {
    /// Create a new parser from source code
    pub fn new(source: &'source str) -> Self {
        Parser {
            lexer: Lexer::new(source),
            seed_counter: 0,
        }
    }

    /// Parse a complete statement (either a mini pattern or command)
    pub fn parse_statement(&mut self) -> Result<Ast> {
        // Try to parse a command first (setcps, setbpm, hush)
        if let Some((token, _)) = self.peek() {
            if matches!(token, Token::Setcps | Token::Setbpm | Token::Hush) {
                return self.parse_command();
            }
        }

        // Otherwise parse as mini notation
        self.parse_mini_definition()
    }

    /// Parse a mini notation pattern (may or may not have quotes)
    pub fn parse_mini_definition(&mut self) -> Result<Ast> {
        // Check if we have quotes
        if let Some((Token::Quote | Token::SingleQuote, _)) = self.peek() {
            self.parse_mini()
        } else {
            // Parse without quotes (bare pattern)
            self.parse_stack_or_choose()
        }
    }

    /// Parse a quoted mini notation string: "pattern"
    fn parse_mini(&mut self) -> Result<Ast> {
        let start_span = self.expect_token(Token::Quote)?;
        let pattern = self.parse_stack_or_choose()?;
        let end_span = self.expect_token(Token::Quote)?;

        let span = start_span.merge(end_span);

        // Wrap in a pattern node if it's not already
        match pattern {
            Ast::Pattern(mut p) => {
                p.span = span;
                Ok(Ast::Pattern(p))
            }
            _ => Ok(pattern),
        }
    }

    /// Parse stack (,), choose (|), or dot (.) separated sequences
    fn parse_stack_or_choose(&mut self) -> Result<Ast> {
        let first = self.parse_sequence()?;

        // Check what comes next
        let separator = match self.peek() {
            Some((Token::Comma, _)) => Some(Alignment::Stack),
            Some((Token::Pipe, _)) => Some(Alignment::Rand),
            Some((Token::Dot, _)) => Some(Alignment::Feet),
            _ => None,
        };

        if let Some(alignment) = separator {
            let mut children = vec![Ast::Pattern(first)];
            let start_span = children[0].span();

            // Consume separator and parse more sequences
            while let Some((token, _)) = self.peek() {
                if !matches!((alignment, token), (Alignment::Stack, Token::Comma) | (Alignment::Rand, Token::Pipe) | (Alignment::Feet, Token::Dot)) {
                    break;
                }

                self.next(); // consume separator
                children.push(Ast::Pattern(self.parse_sequence()?));
            }

            let end_span = children.last().unwrap().span();
            let span = start_span.merge(end_span);

            // For random choice, use a seed
            let seed = if alignment == Alignment::Rand {
                let s = self.seed_counter;
                self.seed_counter += 1;
                Some(s)
            } else {
                None
            };

            Ok(Ast::Pattern(PatternNode::new(
                children,
                alignment,
                seed,
                false,
                span,
            )))
        } else {
            // No separator - check if we should unwrap
            // If the sequence has only one child and it's a non-fastcat pattern, return it directly
            if first.children.len() == 1 {
                if let Ast::Element(elem) = &first.children[0] {
                    if let Ast::Pattern(inner) = elem.source.as_ref() {
                        // Check if this is a special pattern (not fastcat)
                        if !matches!(inner.alignment, Alignment::Fastcat) && elem.ops.is_empty() {
                            return Ok(Ast::Pattern(inner.clone()));
                        }
                    }
                }
            }
            Ok(Ast::Pattern(first))
        }
    }

    /// Parse a sequence (space-separated elements)
    fn parse_sequence(&mut self) -> Result<PatternNode> {
        let mut elements = Vec::new();
        let mut steps_marked = false;

        // Check for step marking (^)
        if let Some((Token::Caret, _)) = self.peek() {
            self.next();
            steps_marked = true;
        }

        let start_span = self.current_span();

        // Parse at least one element
        elements.push(Ast::Element(self.parse_slice_with_ops()?));

        // Parse more elements (space-separated, implicit)
        #[allow(clippy::while_let_loop)]
        loop {
            // Stop at separators or closing delimiters
            if let Some((token, _)) = self.peek() {
                if matches!(
                    token,
                    Token::Comma
                        | Token::Pipe
                        | Token::Dot
                        | Token::RBracket
                        | Token::RBrace
                        | Token::RAngle
                        | Token::RParen
                        | Token::Quote
                        | Token::SingleQuote
                ) {
                    break;
                }
            } else {
                break;
            }

            // Try to parse another element
            match self.parse_slice_with_ops() {
                Ok(elem) => elements.push(Ast::Element(elem)),
                Err(_) => break,
            }
        }

        let end_span = elements.last().map(|e| e.span()).unwrap_or(start_span);
        let span = start_span.merge(end_span);

        Ok(PatternNode::new(
            elements,
            Alignment::Fastcat,
            None,
            steps_marked,
            span,
        ))
    }

    /// Parse a slice with operators (e.g., "bd*2@3")
    fn parse_slice_with_ops(&mut self) -> Result<ElementNode> {
        let slice = self.parse_slice()?;
        let start_span = slice.span();

        let mut element = ElementNode::new(slice, start_span);

        // Parse operators
        loop {
            let op = match self.peek() {
                Some((Token::Star, _)) => {
                    self.next();
                    let amount = Box::new(self.parse_slice()?);
                    Some(SliceOp::Stretch {
                        amount,
                        op_type: StretchType::Fast,
                    })
                }
                Some((Token::Slash, _)) => {
                    self.next();
                    let amount = Box::new(self.parse_slice()?);
                    Some(SliceOp::Stretch {
                        amount,
                        op_type: StretchType::Slow,
                    })
                }
                Some((Token::At, _)) => {
                    self.next();
                    // Weight operator - optional number
                    let amount = if let Some((Token::Number(n), _)) = self.peek() {
                        self.next();
                        n
                    } else {
                        2.0 // default weight increment
                    };
                    element.weight += amount - 1.0;
                    None
                }
                Some((Token::Underscore, _)) => {
                    self.next();
                    // Same as @ operator
                    element.weight += 1.0;
                    None
                }
                Some((Token::Bang, _)) => {
                    self.next();
                    // Replication - optional number
                    let amount = if let Some((Token::Number(n), _)) = self.peek() {
                        self.next();
                        n as usize
                    } else {
                        element.reps + 1 // each ! adds one repetition
                    };
                    element.reps = amount;
                    element.weight = amount as f64;
                    Some(SliceOp::Replicate { amount })
                }
                Some((Token::LParen, _)) => {
                    // Bjorklund (Euclidean rhythm): (pulse, step, rotation?)
                    self.next(); // consume (
                    let pulse = Box::new(Ast::Element(self.parse_slice_with_ops()?));
                    self.expect_token(Token::Comma)?;
                    let step = Box::new(Ast::Element(self.parse_slice_with_ops()?));

                    let rotation = if let Some((Token::Comma, _)) = self.peek() {
                        self.next();
                        Some(Box::new(Ast::Element(self.parse_slice_with_ops()?)))
                    } else {
                        None
                    };

                    self.expect_token(Token::RParen)?;

                    Some(SliceOp::Bjorklund {
                        pulse,
                        step,
                        rotation,
                    })
                }
                Some((Token::Question, _)) => {
                    self.next();
                    // Degrade - optional probability
                    let amount = if let Some((Token::Number(n), _)) = self.peek() {
                        self.next();
                        Some(n)
                    } else {
                        Some(0.5) // default 50%
                    };

                    let seed = self.seed_counter;
                    self.seed_counter += 1;

                    Some(SliceOp::DegradeBy { amount, seed })
                }
                Some((Token::Colon, _)) => {
                    self.next();
                    let element = Box::new(self.parse_slice()?);
                    Some(SliceOp::Tail { element })
                }
                Some((Token::DotDot, _)) => {
                    self.next();
                    let element = Box::new(self.parse_slice()?);
                    Some(SliceOp::Range { element })
                }
                _ => None,
            };

            if let Some(op) = op {
                element.add_op(op);
            } else {
                break;
            }
        }

        element.span = start_span.merge(self.current_span());
        Ok(element)
    }

    /// Parse a slice (atom, sub-cycle, polymeter, or slow sequence)
    fn parse_slice(&mut self) -> Result<Ast> {
        match self.peek() {
            Some((Token::LBracket, _)) => self.parse_sub_cycle(),
            Some((Token::LBrace, _)) => self.parse_polymeter(),
            Some((Token::LAngle, _)) => self.parse_slow_sequence(),
            Some((Token::Tilde | Token::Dash, span)) => {
                self.next();
                Ok(Ast::Atom(AtomNode::silence(span)))
            }
            Some((Token::Number(n), span)) => {
                let num = n;
                let sp = span;
                self.next();
                Ok(Ast::Atom(AtomNode::number(num, sp)))
            }
            Some((Token::Atom, span)) => {
                let sp = span;
                let atom_str = self.lexer.slice(sp).to_string();
                self.next();
                Ok(Ast::Atom(AtomNode::string(atom_str, sp)))
            }
            Some((token, span)) => {
                Err(ParseError::unexpected_token(
                    "atom, number, or opening delimiter",
                    token.to_string(),
                    span,
                ))
            }
            None => Err(ParseError::unexpected_eof("slice")),
        }
    }

    /// Parse a sub-cycle: [pattern]
    fn parse_sub_cycle(&mut self) -> Result<Ast> {
        let start_span = self.expect_token(Token::LBracket)?;
        let pattern = self.parse_stack_or_choose()?;
        let end_span = self.expect_token(Token::RBracket)?;

        // Update span to include brackets
        match pattern {
            Ast::Pattern(mut p) => {
                p.span = start_span.merge(end_span);
                Ok(Ast::Pattern(p))
            }
            other => Ok(other),
        }
    }

    /// Parse a polymeter: {pattern1, pattern2}%steps?
    fn parse_polymeter(&mut self) -> Result<Ast> {
        let start_span = self.expect_token(Token::LBrace)?;

        // Parse comma-separated patterns (polymeter_stack)
        let first = self.parse_sequence()?;
        let mut children = vec![Ast::Pattern(first)];

        while let Some((Token::Comma, _)) = self.peek() {
            self.next();
            children.push(Ast::Pattern(self.parse_sequence()?));
        }

        let end_span = self.expect_token(Token::RBrace)?;

        // Check for explicit steps per cycle (%n)
        let steps_per_cycle = if let Some((Token::Percent, _)) = self.peek() {
            self.next();
            Some(Box::new(self.parse_slice()?))
        } else {
            None
        };

        let span = start_span.merge(end_span);

        let mut pattern = PatternNode::new(children, Alignment::Polymeter, None, false, span);
        if let Some(steps) = steps_per_cycle {
            pattern.steps_per_cycle = Some(steps);
        }

        Ok(Ast::Pattern(pattern))
    }

    /// Parse a slow sequence: <pattern1 pattern2>
    fn parse_slow_sequence(&mut self) -> Result<Ast> {
        let start_span = self.expect_token(Token::LAngle)?;

        // Parse as polymeter_stack (comma-separated sequences)
        let first = self.parse_sequence()?;
        let mut children = vec![Ast::Pattern(first)];

        while let Some((Token::Comma, _)) = self.peek() {
            self.next();
            children.push(Ast::Pattern(self.parse_sequence()?));
        }

        let end_span = self.expect_token(Token::RAngle)?;
        let span = start_span.merge(end_span);

        Ok(Ast::Pattern(PatternNode::new(
            children,
            Alignment::PolymeterSlowcat,
            None,
            false,
            span,
        )))
    }

    /// Parse a command (setcps, setbpm, hush)
    fn parse_command(&mut self) -> Result<Ast> {
        let (token, span) = self.next().ok_or(ParseError::unexpected_eof("command"))?;

        let cmd_type = match token {
            Token::Setcps => CommandType::Setcps,
            Token::Setbpm => CommandType::Setbpm,
            Token::Hush => {
                return Ok(Ast::Command(CommandNode::new(CommandType::Hush, None, span)));
            }
            _ => {
                return Err(ParseError::unexpected_token("command", token.to_string(), span));
            }
        };

        // Parse the number argument
        let (token, num_span) = self.next().ok_or(ParseError::unexpected_eof("number"))?;
        let value = match token {
            Token::Number(n) => n,
            _ => {
                return Err(ParseError::unexpected_token(
                    "number",
                    token.to_string(),
                    num_span,
                ));
            }
        };

        // Convert BPM to CPS if needed
        let final_value = if matches!(cmd_type, CommandType::Setbpm) {
            value / 120.0 / 2.0 // BPM to CPS conversion
        } else {
            value
        };

        let final_span = span.merge(num_span);
        Ok(Ast::Command(CommandNode::new(
            cmd_type,
            Some(final_value),
            final_span,
        )))
    }

    // Helper methods

    fn peek(&mut self) -> Option<(Token, Span)> {
        self.lexer.peek_token()
    }

    fn next(&mut self) -> Option<(Token, Span)> {
        self.lexer.next_token()
    }

    fn expect_token(&mut self, expected: Token) -> Result<Span> {
        match self.next() {
            Some((token, span)) if token == expected => Ok(span),
            Some((token, span)) => Err(ParseError::unexpected_token(
                expected.to_string(),
                token.to_string(),
                span,
            )),
            None => Err(ParseError::unexpected_eof(expected.to_string())),
        }
    }

    fn current_span(&self) -> Span {
        // Return a zero-width span at current position
        // This is a fallback for when we need a span but don't have one
        Span::new(0, 0)
    }
}

/// Convenience function to parse a mini notation string
pub fn parse(source: &str) -> Result<Ast> {
    let mut parser = Parser::new(source);
    parser.parse_statement()
}

/// Parse a mini notation pattern (with or without quotes)
pub fn parse_mini(source: &str) -> Result<Ast> {
    let mut parser = Parser::new(source);
    parser.parse_mini_definition()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_atom() {
        let ast = parse("bd").unwrap();
        match ast {
            Ast::Pattern(p) => {
                assert_eq!(p.alignment, Alignment::Fastcat);
                assert_eq!(p.children.len(), 1);
            }
            _ => panic!("Expected Pattern"),
        }
    }

    #[test]
    fn test_parse_sequence() {
        let ast = parse("bd sd cp").unwrap();
        match ast {
            Ast::Pattern(p) => {
                assert_eq!(p.alignment, Alignment::Fastcat);
                assert_eq!(p.children.len(), 3);
            }
            _ => panic!("Expected Pattern"),
        }
    }

    #[test]
    fn test_parse_silence() {
        let ast = parse("~ bd -").unwrap();
        match ast {
            Ast::Pattern(p) => {
                assert_eq!(p.children.len(), 3);
                match &p.children[0] {
                    Ast::Element(e) => match e.source.as_ref() {
                        Ast::Atom(a) => assert_eq!(a.value, AtomValue::Silence),
                        _ => panic!("Expected Atom"),
                    },
                    _ => panic!("Expected Element"),
                }
            }
            _ => panic!("Expected Pattern"),
        }
    }

    #[test]
    fn test_parse_brackets() {
        let ast = parse("bd [sd cp]").unwrap();
        match ast {
            Ast::Pattern(p) => {
                assert_eq!(p.children.len(), 2);
                // Second child should be a pattern
                match &p.children[1] {
                    Ast::Element(e) => match e.source.as_ref() {
                        Ast::Pattern(inner) => {
                            assert_eq!(inner.children.len(), 2);
                        }
                        _ => panic!("Expected Pattern inside element"),
                    },
                    _ => panic!("Expected Element"),
                }
            }
            _ => panic!("Expected Pattern"),
        }
    }

    #[test]
    fn test_parse_fast_operator() {
        let ast = parse("bd*2").unwrap();
        match ast {
            Ast::Pattern(p) => {
                assert_eq!(p.children.len(), 1);
                match &p.children[0] {
                    Ast::Element(e) => {
                        assert_eq!(e.ops.len(), 1);
                        match &e.ops[0] {
                            SliceOp::Stretch { op_type, .. } => {
                                assert_eq!(*op_type, StretchType::Fast);
                            }
                            _ => panic!("Expected Stretch op"),
                        }
                    }
                    _ => panic!("Expected Element"),
                }
            }
            _ => panic!("Expected Pattern"),
        }
    }

    #[test]
    fn test_parse_stack() {
        let ast = parse("bd,sd,cp").unwrap();
        match ast {
            Ast::Pattern(p) => {
                assert_eq!(p.alignment, Alignment::Stack);
                assert_eq!(p.children.len(), 3);
            }
            _ => panic!("Expected Pattern"),
        }
    }

    #[test]
    fn test_parse_choose() {
        let ast = parse("bd|sd|cp").unwrap();
        match ast {
            Ast::Pattern(p) => {
                assert_eq!(p.alignment, Alignment::Rand);
                assert_eq!(p.children.len(), 3);
                assert!(p.seed.is_some());
            }
            _ => panic!("Expected Pattern"),
        }
    }

    #[test]
    fn test_parse_polymeter() {
        let ast = parse("{bd sd, cp hh oh}").unwrap();
        match ast {
            Ast::Pattern(p) => {
                assert_eq!(p.alignment, Alignment::Polymeter);
                assert_eq!(p.children.len(), 2);
            }
            _ => panic!("Expected Pattern"),
        }
    }

    #[test]
    fn test_parse_slow_sequence() {
        let ast = parse("<bd sd cp>").unwrap();
        match ast {
            Ast::Pattern(p) => {
                assert_eq!(p.alignment, Alignment::PolymeterSlowcat);
                assert_eq!(p.children.len(), 1);
            }
            _ => panic!("Expected Pattern"),
        }
    }

    #[test]
    fn test_parse_command_setcps() {
        let ast = parse("setcps 0.5").unwrap();
        match ast {
            Ast::Command(c) => {
                assert_eq!(c.cmd_type, CommandType::Setcps);
                assert_eq!(c.value, Some(0.5));
            }
            _ => panic!("Expected Command"),
        }
    }

    #[test]
    fn test_parse_command_hush() {
        let ast = parse("hush").unwrap();
        match ast {
            Ast::Command(c) => {
                assert_eq!(c.cmd_type, CommandType::Hush);
                assert_eq!(c.value, None);
            }
            _ => panic!("Expected Command"),
        }
    }
}
