use crate::span::Span;
use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]  // Skip whitespace
pub enum Token {
    // Numbers - highest priority to avoid conflicts
    #[regex(r"-?[0-9]+\.?[0-9]*([eE][+-]?[0-9]+)?", parse_number, priority = 10)]
    Number(f64),

    // Atoms - letters and combinations, but not standalone operators
    #[regex(r"[a-zA-Z][a-zA-Z0-9_#.^~-]*|[a-zA-Z0-9]+[_#.^~-]+[a-zA-Z0-9_#.^~-]*", priority = 5)]
    Atom,

    // Delimiters
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("<")]
    LAngle,
    #[token(">")]
    RAngle,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("\"")]
    Quote,
    #[token("'")]
    SingleQuote,

    // Separators
    #[token(",")]
    Comma,
    #[token("|")]
    Pipe,
    #[token(".")]
    Dot,

    // Operators
    #[token("@")]
    At,
    #[token("_")]
    Underscore,
    #[token("!")]
    Bang,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("?")]
    Question,
    #[token(":")]
    Colon,
    #[token("..")]
    DotDot,
    #[token("%")]
    Percent,
    #[token("^")]
    Caret,
    #[token("$")]
    Dollar,

    // Special atoms
    #[token("~")]
    Tilde,
    #[token("-")]
    Dash,

    // Keywords
    #[token("setcps")]
    Setcps,
    #[token("setbpm")]
    Setbpm,
    #[token("hush")]
    Hush,
    #[token("slow")]
    Slow,
    #[token("fast")]
    Fast,
    #[token("scale")]
    Scale,
    #[token("struct")]
    Struct,
    #[token("target")]
    Target,
    #[token("euclid")]
    Euclid,
    #[token("rotL")]
    RotL,
    #[token("rotR")]
    RotR,
    #[token("cat")]
    Cat,

    // Comments
    #[regex(r"//[^\n]*")]
    Comment,

    // Error token
    Error,
}

fn parse_number(lex: &mut logos::Lexer<Token>) -> Option<f64> {
    let slice = lex.slice();
    slice.parse().ok()
}

impl Token {
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Token::Setcps
                | Token::Setbpm
                | Token::Hush
                | Token::Slow
                | Token::Fast
                | Token::Scale
                | Token::Struct
                | Token::Target
                | Token::Euclid
                | Token::RotL
                | Token::RotR
                | Token::Cat
        )
    }

    pub fn is_delimiter(&self) -> bool {
        matches!(
            self,
            Token::LBracket
                | Token::RBracket
                | Token::LBrace
                | Token::RBrace
                | Token::LAngle
                | Token::RAngle
                | Token::LParen
                | Token::RParen
        )
    }

    pub fn is_operator(&self) -> bool {
        matches!(
            self,
            Token::At
                | Token::Underscore
                | Token::Bang
                | Token::Star
                | Token::Slash
                | Token::Question
                | Token::Colon
                | Token::DotDot
                | Token::Percent
                | Token::Caret
        )
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Number(n) => write!(f, "{}", n),
            Token::Atom => write!(f, "atom"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LAngle => write!(f, "<"),
            Token::RAngle => write!(f, ">"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::Quote => write!(f, "\""),
            Token::SingleQuote => write!(f, "'"),
            Token::Comma => write!(f, ","),
            Token::Pipe => write!(f, "|"),
            Token::Dot => write!(f, "."),
            Token::At => write!(f, "@"),
            Token::Underscore => write!(f, "_"),
            Token::Bang => write!(f, "!"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Question => write!(f, "?"),
            Token::Colon => write!(f, ":"),
            Token::DotDot => write!(f, ".."),
            Token::Percent => write!(f, "%"),
            Token::Caret => write!(f, "^"),
            Token::Dollar => write!(f, "$"),
            Token::Tilde => write!(f, "~"),
            Token::Dash => write!(f, "-"),
            Token::Setcps => write!(f, "setcps"),
            Token::Setbpm => write!(f, "setbpm"),
            Token::Hush => write!(f, "hush"),
            Token::Slow => write!(f, "slow"),
            Token::Fast => write!(f, "fast"),
            Token::Scale => write!(f, "scale"),
            Token::Struct => write!(f, "struct"),
            Token::Target => write!(f, "target"),
            Token::Euclid => write!(f, "euclid"),
            Token::RotL => write!(f, "rotL"),
            Token::RotR => write!(f, "rotR"),
            Token::Cat => write!(f, "cat"),
            Token::Comment => write!(f, "comment"),
            Token::Error => write!(f, "error"),
        }
    }
}

/// Lexer wrapper with position tracking
pub struct Lexer<'source> {
    inner: logos::Lexer<'source, Token>,
    peeked: Option<Option<(Token, Span)>>,
}

impl<'source> Lexer<'source> {
    pub fn new(source: &'source str) -> Self {
        Lexer {
            inner: Token::lexer(source),
            peeked: None,
        }
    }

    pub fn next_token(&mut self) -> Option<(Token, Span)> {
        if let Some(peeked) = self.peeked.take() {
            return peeked;
        }

        loop {
            let token = self.inner.next()?;
            let span = Span::from(self.inner.span());

            // Skip comments
            if matches!(token, Ok(Token::Comment)) {
                continue;
            }

            let token = token.unwrap_or(Token::Error);
            return Some((token, span));
        }
    }

    pub fn peek_token(&mut self) -> Option<(Token, Span)> {
        if self.peeked.is_none() {
            self.peeked = Some(self.next_token());
        }
        self.peeked.as_ref().and_then(|x| x.clone())
    }

    pub fn source(&self) -> &'source str {
        self.inner.source()
    }

    pub fn slice(&self, span: Span) -> &'source str {
        &self.source()[span.to_range()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(input: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();
        while let Some((token, _)) = lexer.next_token() {
            tokens.push(token);
        }
        tokens
    }

    #[test]
    fn test_lex_atoms() {
        let tokens = lex("bd sd cp hh");
        assert_eq!(
            tokens,
            vec![Token::Atom, Token::Atom, Token::Atom, Token::Atom]
        );
    }

    #[test]
    fn test_lex_numbers() {
        let tokens = lex("1 2.5 -3 4e2");
        assert_eq!(
            tokens,
            vec![
                Token::Number(1.0),
                Token::Number(2.5),
                Token::Number(-3.0),
                Token::Number(400.0)
            ]
        );
    }

    #[test]
    fn test_lex_brackets() {
        let tokens = lex("[bd sd]");
        assert_eq!(
            tokens,
            vec![Token::LBracket, Token::Atom, Token::Atom, Token::RBracket]
        );
    }

    #[test]
    fn test_lex_operators() {
        let tokens = lex("bd*2 sd@3 cp?");
        assert_eq!(
            tokens,
            vec![
                Token::Atom,
                Token::Star,
                Token::Number(2.0),
                Token::Atom,
                Token::At,
                Token::Number(3.0),
                Token::Atom,
                Token::Question
            ]
        );
    }

    #[test]
    fn test_lex_silence() {
        let tokens = lex("~ -");
        assert_eq!(tokens, vec![Token::Tilde, Token::Dash]);
    }

    #[test]
    fn test_lex_keywords() {
        let tokens = lex("setcps 0.5");
        assert_eq!(tokens, vec![Token::Setcps, Token::Number(0.5)]);
    }

    #[test]
    fn test_lex_complex() {
        let tokens = lex("bd(3,8) [sd,cp]*2");
        assert_eq!(
            tokens,
            vec![
                Token::Atom,
                Token::LParen,
                Token::Number(3.0),
                Token::Comma,
                Token::Number(8.0),
                Token::RParen,
                Token::LBracket,
                Token::Atom,
                Token::Comma,
                Token::Atom,
                Token::RBracket,
                Token::Star,
                Token::Number(2.0)
            ]
        );
    }

    #[test]
    fn test_lex_skip_comments() {
        let tokens = lex("bd // comment\nsd");
        assert_eq!(tokens, vec![Token::Atom, Token::Atom]);
    }

    #[test]
    fn test_lexer_slice() {
        let input = "bd sd cp";
        let mut lexer = Lexer::new(input);

        let (token, span) = lexer.next_token().unwrap();
        assert_eq!(token, Token::Atom);
        assert_eq!(lexer.slice(span), "bd");

        let (token, span) = lexer.next_token().unwrap();
        assert_eq!(token, Token::Atom);
        assert_eq!(lexer.slice(span), "sd");
    }

    #[test]
    fn test_lexer_peek() {
        let mut lexer = Lexer::new("bd sd");

        let (token, _) = lexer.peek_token().unwrap();
        assert_eq!(token, Token::Atom);

        // Peek again - should be same
        let (token, _) = lexer.peek_token().unwrap();
        assert_eq!(token, Token::Atom);

        // Now consume it
        let (token, _) = lexer.next_token().unwrap();
        assert_eq!(token, Token::Atom);

        // Next token should be different
        let (token, _) = lexer.next_token().unwrap();
        assert_eq!(token, Token::Atom);
    }
}
