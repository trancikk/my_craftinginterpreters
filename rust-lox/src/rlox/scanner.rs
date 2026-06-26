use crate::rlox::interpreter::RLox;
use crate::rlox::token::{Token, TokenType};
use std::str::Lines;

pub struct Scanner<'a> {
    lines: Lines<'a>,
    current_line: Option<&'a str>,
    current_position: usize,
    line_index: u64,
    interpreter: RLox,
    end_is_reached: bool,
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
            end_is_reached: false,
        }
    }

    pub fn next_line(&mut self) -> Option<&str> {
        let next_line = self.lines.next();
        self.current_line = next_line;
        if let Some(_) = next_line {
            self.line_index += 1;
            self.current_position = 0;
        } else {
            self.end_is_reached = true
        }

        next_line
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

    fn advance_by_char(&mut self, ch: &char) {
        self.current_position += ch.len_utf8();
    }

    fn conditional_advance_until(&mut self, ch: char) -> Option<char> {
        let next_char = self.peek()?;
        if next_char != ch {
            self.advance_by_char(&next_char);
            Some(next_char)
        } else {
            None
        }
    }

    fn advance_chars_until(&mut self, ch: char, inclusive: bool) -> Vec<char> {
        let mut buf = vec![];
        'outer: while let Some(line) = self.current_line {
            let chars = line[self.current_position..].chars();
            for c in chars {
                if c != ch {
                    buf.push(c);
                    self.advance_by_char(&c);
                } else {
                    if inclusive {
                        self.advance_by_char(&c);
                    }
                    break 'outer;
                }
            }
            self.next_line();
            buf.push('\n')
        }

        buf
    }

    fn step_back(&mut self) {
        self.current_position -= 1
    }
    fn advance_chars_until_whitespace(&mut self) -> Vec<char> {
        if let Some(line) = self.current_line {
            let chars: Vec<char> = line[self.current_position..]
                .chars()
                .take_while(|c| {
                    self.advance_by_char(c);
                    !c.is_whitespace()
                })
                .collect();
            chars
        } else {
            vec![]
        }
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
            '\"' => {
                let str = self.advance_chars_until('\"', true);
                if self.end_is_reached {
                    self.interpreter.error(
                        self.line_index,
                        &format!("Unterminated string, {:?}", String::from_iter(str)),
                    );
                    None
                } else {
                    let lexeme = String::from_iter(str);
                    Some(Token::new(
                        TokenType::StringToken(lexeme.clone()),
                        //TODO clarify whats lexeme
                        lexeme,
                        self.line_index,
                    ))
                }
            }
            digit if digit >= '0' && digit <= '9' => {
                self.step_back();
                let digits = self.advance_chars_until_whitespace();
                let digits: String = String::from_iter(digits);
                if let Ok(number) = digits.parse::<i64>() {
                    Some(Token::new(
                        TokenType::Number(number),
                        number.to_string(),
                        self.line_index,
                    ))
                } else if let Ok(number) = digits.parse::<f64>() {
                    Some(Token::new(
                        TokenType::FloatNumber(number),
                        number.to_string(),
                        self.line_index,
                    ))
                } else {
                    self.interpreter
                        .error(self.line_index, &format!("Incorrect number! {:?}", digits));
                    self.scan_token()
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
    use crate::rlox::token::TokenType::{FloatNumber, Number};

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
        let interpreter = RLox { had_error: false };
        let prompt = "+ -
        != \" bla bla bla
        ";
        let mut scanner = Scanner::new(interpreter, prompt.lines());
        let _: Vec<_> = scanner.collect_tokens();
        assert!(scanner.interpreter.had_error);
    }

    #[test]
    fn terminated_string_dont_throw_error() {
        let interpreter = RLox { had_error: false };
        let prompt = "+ -
        != \" bla bla bla
        another bla \" =";
        let mut scanner = Scanner::new(interpreter, prompt.lines());
        let result: Vec<_> = scanner.collect_tokens();
        assert!(!scanner.interpreter.had_error);
        for token in result {
            println!("{:?}", token)
        }
    }

    #[test]
    fn scans_integers() {
        //TODO never finishes, scan for string terminator probably has a bug
        let interpreter = RLox { had_error: false };
        let prompt = "
        589565
        ";
        let mut scanner = Scanner::new(interpreter, prompt.lines());
        let result: Vec<_> = scanner.collect_tokens();
        assert!(!scanner.interpreter.had_error);
        for token in result {
            assert!(match token {
                Token { token_type, .. } => {
                    match token_type {
                        Number(value) => value == 589565,
                        _ => false,
                    }
                }
            });
        }
    }

    #[test]
    fn scans_number_fails() {
        //TODO never finishes, scan for string terminator probably has a bug
        let interpreter = RLox { had_error: false };
        let prompt = "
        589565,22
        ";
        let mut scanner = Scanner::new(interpreter, prompt.lines());
        let _: Vec<_> = scanner.collect_tokens();
        assert!(scanner.interpreter.had_error);

    }

    #[test]
    fn scans_floats_dot() {
        //TODO never finishes, scan for string terminator probably has a bug
        let interpreter = RLox { had_error: false };
        let prompt = "
        589565.22
        ";
        let mut scanner = Scanner::new(interpreter, prompt.lines());
        let result: Vec<_> = scanner.collect_tokens();
        assert!(!scanner.interpreter.had_error);
        for token in result {
            assert!(match token {
                Token { token_type, .. } => {
                    match token_type {
                        FloatNumber(value) => value == 589565.22,
                        _ => false,
                    }
                }
            });
        }
    }
}
