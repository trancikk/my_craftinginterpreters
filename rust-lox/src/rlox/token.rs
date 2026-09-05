use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    // TODO embed into token type enum value
    // literal: &'static dyn Any,
    pub line: u64,
}

pub const RESERVED_KEYWORDS: LazyLock<HashMap<String, TokenType>> = LazyLock::new(|| {
    HashMap::from([
        ("and", TokenType::And),
        ("class", TokenType::Class),
        ("else", TokenType::Else),
        ("false", TokenType::False),
        ("for", TokenType::For),
        ("fun", TokenType::Fun),
        ("if", TokenType::If),
        ("nil", TokenType::Nil),
        ("or", TokenType::Or),
        ("print", TokenType::Print),
        ("return", TokenType::Return),
        ("super", TokenType::Super),
        ("this", TokenType::This),
        ("true", TokenType::True),
        ("var", TokenType::Var),
        ("while", TokenType::While),
    ])
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
});

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

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Single char token
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Question,
    Colon,
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
    StringToken(String),
    Number(u64),
    FloatNumber(f64),

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
    This,
    True,
    Var,
    While,

    EOF,
}
