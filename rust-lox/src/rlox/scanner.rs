use crate::rlox::interpreter::RLox;
use crate::rlox::token::{Token, TokenType};
use std::str::Chars;

pub struct Scanner {
    line: String,
    current_position: usize,
    line_index: u64,
    interpreter: RLox,
}

impl Scanner {
    pub fn new(interpreter: RLox, line: String) -> Self {
        Scanner {
            line: line.clone(),
            current_position: 0,
            line_index: 0,
            interpreter,
        }
    }

    pub fn use_line(&mut self, line: String) {
        self.line = line;
        self.line_index += 1;
        self.current_position = 0;
    }

    fn peek(&self) -> Option<char> {
        self.line[self.current_position..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.current_position += ch.len_utf8();
        Some(ch)
    }

    fn conditional_advance(&mut self, ch: char) -> Option<char> {
        let next_char = self.peek()?;
        if next_char == ch {
            self.current_position += next_char.len_utf8();
            Some(next_char)
        } else {
            None
        }
    }

    fn conditional_advance_vec(&mut self, chars: &[char]) -> Option<char> {
        let next_char = self.peek()?;
        for ch in chars {
            if next_char == *ch {
                self.current_position += next_char.len_utf8();
                return Some(next_char);
            }
        }
        None
    }

    pub fn scan_token(&mut self) -> Option<Token> {
        let ch = self.advance()?;
        match ch {
            '(' => Some(Token::new(
                TokenType::LeftParen,
                ch.to_string(),
                self.line_index,
            )),
            ')' => Some(Token::new(
                TokenType::RightParen,
                ch.to_string(),
                self.line_index,
            )),
            '{' => Some(Token::new(
                TokenType::LeftBrace,
                ch.to_string(),
                self.line_index,
            )),
            '}' => Some(Token::new(
                TokenType::RightBrace,
                ch.to_string(),
                self.line_index,
            )),
            ',' => Some(Token::new(
                TokenType::Comma,
                ch.to_string(),
                self.line_index,
            )),
            '.' => Some(Token::new(TokenType::Dot, ch.to_string(), self.line_index)),
            '-' => Some(Token::new(
                TokenType::Minus,
                ch.to_string(),
                self.line_index,
            )),
            '+' => Some(Token::new(TokenType::Plus, ch.to_string(), self.line_index)),
            ';' => Some(Token::new(
                TokenType::Semicolon,
                ch.to_string(),
                self.line_index,
            )),
            '*' => Some(Token::new(TokenType::Star, ch.to_string(), self.line_index)),
            '!' => {
                let next_char = self.conditional_advance('=');
                match next_char {
                    Some(_) => Some(Token::new(
                        TokenType::BangEqual,
                        "!=".into(),
                        self.line_index,
                    )),
                    None => Some(Token::new(TokenType::Bang, ch.to_string(), self.line_index)),
                }
            }
            '=' => {
                let next_char = self.conditional_advance('=');
                match next_char {
                    Some(_) => Some(Token::new(
                        TokenType::EqualEqual,
                        "==".into(),
                        self.line_index,
                    )),
                    None => Some(Token::new(
                        TokenType::Equal,
                        ch.to_string(),
                        self.line_index,
                    )),
                }
            }
            '<' => {
                let next_char = self.conditional_advance('=');
                match next_char {
                    Some(_) => Some(Token::new(
                        TokenType::LessEqual,
                        "<=".into(),
                        self.line_index,
                    )),
                    None => Some(Token::new(TokenType::Less, ch.to_string(), self.line_index)),
                }
            }
            '>' => {
                let next_char = self.conditional_advance('=');
                match next_char {
                    Some(_) => Some(Token::new(
                        TokenType::GreaterEqual,
                        ">=".into(),
                        self.line_index,
                    )),
                    None => Some(Token::new(
                        TokenType::Greater,
                        ch.to_string(),
                        self.line_index,
                    )),
                }
            }
            _ => {
                self.interpreter
                    .error(self.line_index, "Unexpected character");
                None
            }
        }
    }
}

impl Iterator for Scanner {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        self.scan_token()
    }
}
