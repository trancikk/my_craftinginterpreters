use crate::rlox::interpreter::RLox;
use crate::rlox::token::{Token, TokenType};
use std::str::Lines;

pub struct Scanner<'a> {
    lines: Lines<'a>,
    current_line: Option<&'a str>,
    current_position: usize,
    line_index: u64,
    interpreter: RLox,
}

impl<'a> Scanner<'a> {
    pub fn new(interpreter: RLox, mut lines: Lines<'a>) -> Self {
        let current_line = lines.next();
        Scanner {
            lines,
            current_line,
            current_position: 0,
            line_index: 0,
            interpreter,
        }
    }

    pub fn next_line(&mut self) -> Option<&str> {
        let next_line = self.lines.next();
        match next_line {
            Some(_) => {
                self.current_line = next_line;
                self.line_index += self.line_index;
                self.current_position = 0;
                self.current_line
            }
            None => None,
        }
    }

    fn peek(&self) -> Option<char> {
        self.current_line?[self.current_position..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek();
        match ch {
            Some(c) => {
                self.current_position += c.len_utf8();
                ch
            }
            None => {
                self.next_line()?;
                self.advance()
            }
        }
    }

    fn advance_by_char(&mut self, ch: char) {
        self.current_position += ch.len_utf8();
    }

    fn conditional_advance_until(&mut self, ch: char) -> Option<char> {
        let next_char = self.peek()?;
        if next_char != ch {
            self.advance_by_char(next_char);
            Some(next_char)
        } else {
            None
        }
    }

    fn advance_chars_until(&mut self, ch: char) -> Option<Vec<char>> {
        let mut buf = vec![];
        'outer: while let Some(line) = self.current_line {
            let chars = line[self.current_position..].chars();
            for c in chars {
                if c != ch {
                    buf.push(c);
                    self.advance_by_char(c)
                } else {
                    break 'outer;
                }
            }
            self.next_line();
            buf.push('\n')
        }

        Some(buf.into())
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
            '/' => {
                if let Some(_next_char) = self.conditional_advance('/') {
                    while let Some(_next) = self.conditional_advance_until('\n') {}
                    self.scan_token()
                } else {
                    Some(Token::new(
                        TokenType::Slash,
                        ch.to_string(),
                        self.line_index,
                    ))
                }
            }
            ' ' | '\r' | '\t' => self.scan_token(),
            '\n' => {
                self.next_line();
                self.scan_token()
            }
            '\'' => {
                let str = self.advance_chars_until('\'');
                match str {
                    Some(s) => {
                        let lexeme = String::from_iter(s);
                        Some(Token::new(
                            TokenType::StringToken(lexeme.clone()),
                            //TODO clarify whats lexeme
                            lexeme,
                            self.line_index,
                        ))
                    }
                    None => {
                        self.interpreter
                            .error(self.line_index, "Unterminated string.");
                        None
                    }
                }
            }
            _ => {
                self.interpreter
                    .error(self.line_index, "Unexpected character");
                None
            }
        }
    }

    pub fn collect_tokens(&mut self) -> Vec<Token> {
        let mut v = vec![];
        for token in self {
            v.push(token)
        }
        v
    }
}

impl Iterator for Scanner<'_> {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        self.scan_token()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let interpreter = RLox { had_error: false };
        let prompt = "+ -
        !=
        ";
        let scanner = Scanner::new(interpreter, prompt.lines());
        for token in scanner {
            println!("{}", token)
        }
    }

    #[test]
    fn unterminated_string_throws_error() {
        //TODO never finishes, scan for string terminator probably has a bug
        let interpreter = RLox { had_error: false };
        let prompt = "+ -
        != ' bla bla bla
        ";
        let mut scanner = Scanner::new(interpreter, prompt.lines());
        let _: Vec<_> = scanner.collect_tokens();
        assert!(scanner.interpreter.had_error);
    }
}
