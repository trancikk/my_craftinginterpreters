use std::any::Any;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct Token {
    token_type: TokenType,
    lexeme: String,
    // TODO embed into token type enum value
    // literal: &'static dyn Any,
    line: u64,
}

impl Token {
    pub fn new(token_type: TokenType, lexeme: String, line: u64) -> Token {
        Token {
            token_type,
            lexeme,
            // literal,
            line,
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {:?}", self.token_type, self.lexeme)
    }
}

#[derive(Debug)]
pub enum TokenType {
    // Single char token
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,

    // one or two char tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals
    Identifier(String),
    String(String),
    Number(u64),

    // Keywords
    And,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    True,
    Var,
    While,
}
