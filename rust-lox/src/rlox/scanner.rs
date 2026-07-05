use crate::rlox::interpreter::RLox;
use crate::rlox::token::{RESERVED_KEYWORDS, Token, TokenType};
use std::str::{Chars, Lines};

pub struct Scanner<'a> {
    lines: Lines<'a>,
    current_line: Option<&'a str>,
    current_position: usize,
    pub line_index: u64,
    end_is_reached: bool,
}

impl<'a> Scanner<'a> {
    pub fn new(mut lines: Lines<'a>) -> Self {
        let current_line = lines.next();
        Scanner {
            lines,
            current_line,
            current_position: 0,
            line_index: 0,
            end_is_reached: false,
        }
    }

    pub fn next_line(&mut self) -> Option<&str> {
        let next_line = self.lines.next();
        self.current_line = next_line;
        if next_line.is_some() {
            self.line_index += 1;
            self.current_position = 0;
        } else {
            self.end_is_reached = true;
        }

        next_line
    }

    fn peek(&self) -> Option<char> {
        self.current_line?[self.current_position..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        loop {
            let ch = self.advance_in_line();
            if ch.is_none() {
                self.next_line()?;
            } else {
                return ch;
            }
        }
    }

    fn advance_by_char(&mut self, ch: char) {
        self.current_position += ch.len_utf8();
    }

    fn advance_in_line(&mut self) -> Option<char> {
        let ch = self.peek();
        if let Some(next_char) = ch {
            self.advance_by_char(next_char);
        }
        ch
    }

    fn get_current_line_cursor(&self) -> Option<Chars> {
        let line = self.current_line?;
        Some(line[self.current_position..].chars())
    }

    fn advance_until_char_in_line(&mut self, ch: char) -> Option<char> {
        let chars = self.get_current_line_cursor()?;
        for next_char in chars {
            if next_char == ch {
                self.advance_by_char(next_char);
                return Some(next_char);
            }
        }
        None
    }

    fn advance_until_char(&mut self, ch: char) -> Option<char> {
        loop {
            let next_char = self.advance_until_char_in_line(ch);
            if next_char.is_none() {
                self.next_line()?;
            } else {
                return next_char;
            }
        }
    }

    fn advance_chars_until(&mut self, ch: char, inclusive: bool) -> Vec<char> {
        let mut buf = vec![];
        'outer: while let Some(line) = self.current_line {
            let chars = line[self.current_position..].chars();
            for c in chars {
                if c == ch {
                    if inclusive {
                        self.advance_by_char(c);
                    }
                    break 'outer;
                }
                buf.push(c);
                self.advance_by_char(c);
            }
            self.next_line();
            buf.push('\n');
        }

        buf
    }

    fn advance_chars_until_fn<F>(&mut self, predicate: F) -> Vec<char>
    where
        F: Fn(char) -> bool,
    {
        let mut buf = vec![];
        'outer: while let Some(line) = self.current_line {
            let chars = line[self.current_position..].chars();
            for c in chars {
                if predicate(c) {
                    break 'outer;
                }
                buf.push(c);
                self.advance_by_char(c);
            }
            self.next_line();
            buf.push('\n');
        }

        buf
    }

    fn step_back(&mut self) {
        self.current_position -= 1;
    }

    fn advance_chars_until_whitespace(&mut self) -> Vec<char> {
        let mut result = vec![];
        // WA for borrowing
        // let line = self.get_current_line_cursor();
        let line = self.current_line;
        if let Some(line) = line {
            let chars = line[self.current_position..].chars();
            for ch in chars {
                self.advance_by_char(ch);
                if ch.is_whitespace() {
                    break;
                }
                result.push(ch);
            }
        }

        result
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

    pub fn scan_token(&mut self, interpreter: &mut RLox) -> Option<Token> {
        'outer: loop {
            let ch = self.advance()?;
            match ch {
                '(' => {
                    return Some(Token::new(
                        TokenType::LeftParen,
                        ch.to_string(),
                        self.line_index,
                    ));
                }
                ')' => {
                    return Some(Token::new(
                        TokenType::RightParen,
                        ch.to_string(),
                        self.line_index,
                    ));
                }
                '{' => {
                    return Some(Token::new(
                        TokenType::LeftBrace,
                        ch.to_string(),
                        self.line_index,
                    ));
                }
                '}' => {
                    return Some(Token::new(
                        TokenType::RightBrace,
                        ch.to_string(),
                        self.line_index,
                    ));
                }
                ',' => {
                    return Some(Token::new(
                        TokenType::Comma,
                        ch.to_string(),
                        self.line_index,
                    ));
                }
                '.' => return Some(Token::new(TokenType::Dot, ch.to_string(), self.line_index)),
                '-' => {
                    return Some(Token::new(
                        TokenType::Minus,
                        ch.to_string(),
                        self.line_index,
                    ));
                }
                '+' => return Some(Token::new(TokenType::Plus, ch.to_string(), self.line_index)),
                ';' => {
                    return Some(Token::new(
                        TokenType::Semicolon,
                        ch.to_string(),
                        self.line_index,
                    ));
                }
                '*' => return Some(Token::new(TokenType::Star, ch.to_string(), self.line_index)),
                '!' => {
                    return match self.conditional_advance('=') {
                        Some(_) => Some(Token::new(
                            TokenType::BangEqual,
                            "!=".into(),
                            self.line_index,
                        )),
                        None => Some(Token::new(TokenType::Bang, ch.to_string(), self.line_index)),
                    };
                }
                '=' => {
                    return match self.conditional_advance('=') {
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
                    };
                }
                '<' => {
                    return match self.conditional_advance('=') {
                        Some(_) => Some(Token::new(
                            TokenType::LessEqual,
                            "<=".into(),
                            self.line_index,
                        )),
                        None => Some(Token::new(TokenType::Less, ch.to_string(), self.line_index)),
                    };
                }
                '>' => {
                    return match self.conditional_advance('=') {
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
                    };
                }
                '/' => {
                    let next_char = self.advance_in_line();
                    match next_char {
                        Some('/') => {
                            self.next_line()?;
                        }
                        Some('*') => loop {
                            self.advance_until_char('*');
                            let next_token = self.advance()?;
                            if next_token == '/' {
                                continue 'outer;
                            }
                        },
                        Some(_) | None => {
                            return Some(Token::new(
                                TokenType::Slash,
                                ch.to_string(),
                                self.line_index,
                            ));
                        }
                    }
                }
                ' ' | '\r' | '\t' => {}
                '\n' => {
                    self.next_line();
                }
                '\"' => {
                    let str = self.advance_chars_until('\"', true);
                    return if self.end_is_reached {
                        interpreter.error(
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
                    };
                }
                digit if digit.is_numeric() => {
                    self.step_back();
                    let digits = self.advance_chars_until_fn(|x| !x.is_numeric());
                    let digits: String = String::from_iter(digits);
                    if let Ok(number) = digits.parse::<u64>() {
                        return Some(Token::new(
                            TokenType::Number(number),
                            number.to_string(),
                            self.line_index,
                        ));
                    } else if let Ok(number) = digits.parse::<f64>() {
                        return Some(Token::new(
                            TokenType::FloatNumber(number),
                            number.to_string(),
                            self.line_index,
                        ));
                    }
                    interpreter.error(self.line_index, &format!("Incorrect number! {digits:?}"));
                }
                alpha if alpha.is_alphabetic() => {
                    self.step_back();
                    let identifier = self.advance_chars_until_whitespace();
                    let value: String = identifier.iter().collect();
                    return if let Some(keyword) = RESERVED_KEYWORDS.get(&value).cloned() {
                        Some(Token::new(keyword, value, self.line_index))
                    } else {
                        Some(Token::new(
                            TokenType::Identifier(value.clone()),
                            value,
                            self.line_index,
                        ))
                    };
                }
                _ => {
                    interpreter.error(self.line_index, "Unexpected character");
                    return None;
                }
            }
        }
    }

    pub fn collect_tokens(&mut self, interpreter: &mut RLox) -> Vec<Token> {
        let mut v = vec![];
        while let Some(token) = self.scan_token(interpreter) {
            v.push(token);
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rlox::token::TokenType::{FloatNumber, Identifier, Number, While};

    struct ScannerIterator<'a> {
        scanner: Scanner<'a>,
        interpreter: &'a mut RLox,
    }

    impl<'a> ScannerIterator<'a> {
        pub fn new(interpreter: &'a mut RLox, scanner: Scanner<'a>) -> ScannerIterator<'a> {
            ScannerIterator {
                scanner,
                interpreter,
            }
        }
    }
    impl Iterator for ScannerIterator<'_> {
        type Item = Token;
        fn next(&mut self) -> Option<Self::Item> {
            self.scanner.scan_token(self.interpreter)
        }
    }

    #[test]
    fn it_works() {
        let mut interpreter = RLox { had_error: false };
        let prompt = "+ -
        !=
        ";
        let scanner = Scanner::new(prompt.lines());
        let scanner_iterator = ScannerIterator::new(&mut interpreter, scanner);
        for token in scanner_iterator {
            println!("{token}");
        }
    }

    #[test]
    fn unterminated_string_throws_error() {
        let mut interpreter = RLox { had_error: false };
        let prompt = "+ -
        != \" bla bla bla
        ";
        let mut scanner = Scanner::new(prompt.lines());
        let _: Vec<_> = scanner.collect_tokens(&mut interpreter);
        assert!(interpreter.had_error);
    }

    #[test]
    fn terminated_string_dont_throw_error() {
        let mut interpreter = RLox { had_error: false };
        let prompt = "+ -
        != \" bla bla bla
        another bla \" =";
        let mut scanner = Scanner::new(prompt.lines());
        let result: Vec<_> = scanner.collect_tokens(&mut interpreter);
        assert!(!interpreter.had_error);
        for token in result {
            println!("{token:?}");
        }
    }

    #[test]
    fn scans_integers() {
        let mut interpreter = RLox { had_error: false };
        let prompt = "
        589565
        ";
        let mut scanner = Scanner::new(prompt.lines());
        let result: Vec<_> = scanner.collect_tokens(&mut interpreter);
        assert!(!interpreter.had_error);
        for token in result {
            assert!({
                let Token { token_type, .. } = token;
                match token_type {
                    Number(value) => value == 589_565,
                    _ => false,
                }
            });
        }
    }

    #[test]
    fn scans_keyword() {
        let mut interpreter = RLox { had_error: false };
        let prompt = "
        while
        ";
        let mut scanner = Scanner::new(prompt.lines());
        let result: Vec<_> = scanner.collect_tokens(&mut interpreter);
        assert!(!interpreter.had_error);
        for token in result {
            assert!({
                let Token { token_type, .. } = token;
                matches!(token_type, While)
            });
        }
    }

    #[test]
    fn scans_identifiers() {
        let mut interpreter = RLox { had_error: false };
        let prompt = "
        my ident
        ";
        let mut scanner = Scanner::new(prompt.lines());
        let result: Vec<_> = scanner.collect_tokens(&mut interpreter);
        assert!(!interpreter.had_error);
        for token in result {
            assert!({
                let Token { token_type, .. } = token;
                match token_type {
                    Identifier(value) => value == "my" || value == "ident",
                    _ => false,
                }
            });
        }
    }

    #[test]
    fn scans_number_fails() {
        let mut interpreter = RLox { had_error: false };
        let prompt = "
        589565,22
        ";
        let mut scanner = Scanner::new(prompt.lines());
        let _: Vec<_> = scanner.collect_tokens(&mut interpreter);
        assert!(interpreter.had_error);
    }

    #[test]
    fn scans_floats_dot() {
        let mut interpreter = RLox { had_error: false };
        let prompt = "
        589565.22
        ";
        let mut scanner = Scanner::new(prompt.lines());
        let result: Vec<_> = scanner.collect_tokens(&mut interpreter);
        assert!(!interpreter.had_error);
        for token in result {
            assert!({
                let Token { token_type, .. } = token;
                match token_type {
                    FloatNumber(value) => value == 589565.22,
                    _ => false,
                }
            });
        }
    }

    #[test]
    fn scans_multiline_comments() {
        let mut interpreter = RLox { had_error: false };
        let prompt = "
        my code /* this is the code
            anoter code *
            end of comment */
        identifier
        ";
        let mut scanner = Scanner::new(prompt.lines());
        let result: Vec<_> = scanner.collect_tokens(&mut interpreter);
        assert!(!interpreter.had_error);
        for token in result {
            assert!({
                let Token { token_type, .. } = token;
                match token_type {
                    Identifier(value) => value == "my" || value == "code" || value == "identifier",
                    _ => false,
                }
            });
        }
    }
}
