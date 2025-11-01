// Extended test cases ported from mini.test.mjs

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use crate::parser::parse;

    // Helper to check if parsing succeeds
    fn assert_parses(input: &str) {
        match parse(input) {
            Ok(_) => (),
            Err(e) => panic!("Failed to parse '{}': {}", input, e),
        }
    }

    // Helper to check parsing fails
    #[allow(dead_code)]
    fn assert_fails(input: &str) {
        if parse(input).is_ok() {
            panic!("Expected parse to fail for '{}'", input)
        }
    }

    #[test]
    fn test_single_elements() {
        assert_parses("a");
        assert_parses("bd");
        assert_parses("42");
        assert_parses("3.14");
    }

    #[test]
    fn test_rest() {
        assert_parses("~");
        assert_parses("-");
        assert_parses("a ~ b");
        assert_parses("a - b");
    }

    #[test]
    fn test_cat() {
        assert_parses("a b");
        assert_parses("a b c");
        assert_parses("bd sd cp hh");
    }

    #[test]
    fn test_fast() {
        assert_parses("a*3 b");
        assert_parses("bd*2");
        assert_parses("bd*4");
    }

    #[test]
    fn test_slow() {
        assert_parses("a/2 b");
        assert_parses("bd/3");
        assert_parses("[a a a]/3 b");
    }

    #[test]
    fn test_brackets() {
        assert_parses("c3 [d3 e3]");
        assert_parses("c3 [d3 [e3 f3]]");
        assert_parses("[a b c]");
        assert_parses("a [b c] d");
    }

    #[test]
    fn test_curly_brackets() {
        assert_parses("{a b, c d e}");
        assert_parses("{a b, c d e}*3");
        assert_parses("{a b, c [d e] f}");
        assert_parses("{a b c, d e}");
    }

    #[test]
    fn test_polymeter_steps() {
        assert_parses("{a b, c d e}%3");
        assert_parses("{a b, c d e}%5");
        assert_parses("{a b, c d e}%6");
    }

    #[test]
    fn test_commas() {
        assert_parses("c3,e3,g3");
        assert_parses("[c3,e3,g3] f3");
        assert_parses("bd,sd");
    }

    #[test]
    fn test_elongation() {
        assert_parses("a@3 b");
        assert_parses("a@2 b@3");
        assert_parses("bd@4 sd");
    }

    #[test]
    fn test_replication() {
        assert_parses("a!3 b");
        assert_parses("bd!2 sd");
        assert_parses("a ! ! b");  // Multiple bangs
        assert_parses("[<a b c>]!3 d");
    }

    #[test]
    fn test_euclidean_rhythms() {
        assert_parses("a(3, 8)");
        assert_parses("bd(3,8)");
        assert_parses("bd(5,16)");
        assert_parses("x(3,8,1)"); // with rotation
    }

    #[test]
    fn test_euclidean_examples() {
        // From Toussaint's paper
        assert_parses("x(1,2)");
        assert_parses("x(1,3)");
        assert_parses("x(1,4)");
        assert_parses("x(4,12)");
        assert_parses("x(2,5)");
        assert_parses("x(3,4)");
        assert_parses("x(3,5)");
        assert_parses("x(3,7)");
        assert_parses("x(3,8)");
        assert_parses("x(4,7)");
        assert_parses("x(4,9)");
        assert_parses("x(4,11)");
        assert_parses("x(5,6)");
        assert_parses("x(5,7)");
        assert_parses("x(5,8)");
        assert_parses("x(5,9)");
        assert_parses("x(5,11)");
        assert_parses("x(5,12)");
        assert_parses("x(5,16)");
        assert_parses("x(7,8)");
        assert_parses("x(7,12)");
        assert_parses("x(7,16)");
        assert_parses("x(9,16)");
        assert_parses("x(11,24)");
        assert_parses("x(13,24)");
    }

    #[test]
    fn test_dash_as_silence() {
        assert_parses("a - b [- c]");
        assert_parses("bd - sd - cp");
    }

    #[test]
    fn test_degrade() {
        assert_parses("a?");
        assert_parses("bd?");
        assert_parses("a?0.8");
        assert_parses("bd?0.3");
    }

    #[test]
    fn test_random_choice() {
        assert_parses("a | b");
        assert_parses("bd | sd | cp");
        assert_parses("a | [b | c] | [d | e | f]");
    }

    #[test]
    fn test_lists() {
        assert_parses("a:b");
        assert_parses("c:d:[e:f]");
        assert_parses("a:b c:d");
    }

    #[test]
    fn test_ranges() {
        assert_parses("0 .. 4");
        assert_parses("1..5");
        assert_parses("0..10");
    }

    #[test]
    fn test_dot_operator() {
        assert_parses("a . b c");
        assert_parses("a . b c . [d e f . g h]");
    }

    #[test]
    fn test_underscore_operator() {
        assert_parses("a _ b _ _");
        assert_parses("bd _ sd _");
    }

    #[test]
    fn test_step_marking() {
        assert_parses("a [^b c]");
        assert_parses("[^b c]!3");
        assert_parses("^[a b c] [d [e f]]");
    }

    #[test]
    fn test_slow_sequence() {
        assert_parses("<a b>");
        assert_parses("<a b c>");
        assert_parses("<a [b c]>");
    }

    #[test]
    fn test_complex_patterns() {
        assert_parses("bd(3,8) [sd,cp]*2 <hh oh>");
        assert_parses("bd*2 [sd cp]*2 hh*4");
        assert_parses("[bd sd, cp hh oh]*2");
        assert_parses("bd sd [cp*2 hh]*2 oh");
    }

    #[test]
    fn test_nested_patterns() {
        assert_parses("[a [b [c [d]]]]");
        assert_parses("a [b c [d e [f g]]]");
        assert_parses("{a [b c], d [e f]}");
    }

    #[test]
    fn test_multiple_operators() {
        assert_parses("bd*2@3");
        assert_parses("sd@2*3");
        assert_parses("cp!3*2");
        assert_parses("bd(3,8)*2");
    }

    #[test]
    fn test_commands() {
        assert_parses("setcps 0.5");
        assert_parses("setbpm 120");
        assert_parses("hush");
    }

    #[test]
    fn test_quoted_patterns() {
        assert_parses("\"bd sd\"");
        assert_parses("\"a b c\"");
        assert_parses("\"bd*2 [sd cp]\"");
    }

    #[test]
    fn test_numbers() {
        assert_parses("1 2 3");
        assert_parses("0.5 1.5 2.5");
        assert_parses("-1 -2 -3");
        assert_parses("1e2 1e-2");
    }

    #[test]
    fn test_whitespace_handling() {
        assert_parses("bd sd cp");
        assert_parses("bd  sd  cp");
        assert_parses("bd\tsd\tcp");
        assert_parses("bd\nsd\ncp");
    }

    #[test]
    fn test_comments() {
        assert_parses("bd // comment");
        assert_parses("bd sd // comment\ncp hh");
    }

    // Parsing counts - verify structure
    #[test]
    fn test_pattern_structure() {
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
    fn test_stack_structure() {
        let ast = parse("a,b,c").unwrap();
        match ast {
            Ast::Pattern(p) => {
                assert_eq!(p.alignment, Alignment::Stack);
                assert_eq!(p.children.len(), 3);
            }
            _ => panic!("Expected Pattern"),
        }
    }

    #[test]
    fn test_operator_structure() {
        let ast = parse("bd*2").unwrap();
        match ast {
            Ast::Pattern(p) => {
                match &p.children[0] {
                    Ast::Element(e) => {
                        assert_eq!(e.ops.len(), 1);
                        match &e.ops[0] {
                            SliceOp::Stretch { op_type, .. } => {
                                assert_eq!(*op_type, StretchType::Fast);
                            }
                            _ => panic!("Expected Stretch"),
                        }
                    }
                    _ => panic!("Expected Element"),
                }
            }
            _ => panic!("Expected Pattern"),
        }
    }
}
