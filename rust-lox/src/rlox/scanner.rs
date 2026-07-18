use crate::rlox::interpreter::RLox;
use crate::rlox::token::{Token, TokenType, RESERVED_KEYWORDS};
use std::str::Lines;

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

    // fn get_current_line_cursor(&self) -> Option<Chars> {
    //     let line = self.current_line?;
    //     Some(line[self.current_position..].chars())
    // }

    fn advance_until_char_in_line(&mut self, ch: char) -> Option<char> {
        let line = self.current_line?;
        for next_char in line[self.current_position..].chars() {
            self.advance_by_char(next_char);
            if next_char == ch {
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
        if let Some(line) = self.current_line {
            let chars = line[self.current_position..].chars();
            for c in chars {
                if predicate(c) {
                    break;
                }
                buf.push(c);
                self.advance_by_char(c);
            }
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
                '?' => {
                    return Some(Token::new(
                        TokenType::Question,
                        ch.to_string(),
                        self.line_index,
                    ));
                }
                ':' => {
                    return Some(Token::new(
                        TokenType::Colon,
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
                    let digits = self.advance_chars_until_fn(|x| !x.is_numeric() && x != '.');
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
    use crate::rlox::token::TokenType::{
        And, Bang, BangEqual, Class, Colon, Comma, Dot, Else, Equal, EqualEqual, False,
        FloatNumber, For, Fun, Greater, GreaterEqual, Identifier, If, LeftBrace, LeftParen, Less,
        LessEqual, Minus, Nil, Number, Or, Plus, Print, Question, Return, RightBrace, RightParen,
        Semicolon, Slash, Star, StringToken, Super, This, True, Var, While,
    };

    fn scan(input: &str) -> (Vec<Token>, RLox) {
        let mut interpreter = RLox { had_error: false };
        let mut scanner = Scanner::new(input.lines());
        let tokens = scanner.collect_tokens(&mut interpreter);
        (tokens, interpreter)
    }

    fn scan_ok(input: &str) -> Vec<Token> {
        let (tokens, interpreter) = scan(input);
        assert!(
            !interpreter.had_error,
            "unexpected scan error for {input:?}"
        );
        tokens
    }

    fn scan_fails(input: &str) {
        let (_, interpreter) = scan(input);
        assert!(interpreter.had_error, "expected scan error for {input:?}");
    }

    fn types(tokens: &[Token]) -> Vec<TokenType> {
        tokens.iter().map(|t| t.token_type.clone()).collect()
    }

    fn assert_single(input: &str, expected: TokenType) {
        let tokens = scan_ok(input);
        assert_eq!(tokens.len(), 1, "input: {input:?}");
        assert_eq!(tokens[0].token_type, expected);
    }

    fn assert_lexeme(input: &str, index: usize, expected: &str) {
        let tokens = scan_ok(input);
        assert_eq!(
            tokens.get(index).map(|t| t.lexeme.as_str()),
            Some(expected),
            "input: {input:?}"
        );
    }

    fn assert_line(input: &str, index: usize, expected: u64) {
        let tokens = scan_ok(input);
        assert_eq!(
            tokens.get(index).map(|t| t.line),
            Some(expected),
            "input: {input:?}"
        );
    }

    mod single_char {
        use super::*;

        #[test]
        fn scans_grouping_and_punctuation() {
            let tokens = scan_ok("( ) { } , . ; ? :");
            assert_eq!(
                types(&tokens),
                vec![
                    LeftParen, RightParen, LeftBrace, RightBrace, Comma, Dot, Semicolon, Question,
                    Colon,
                ]
            );
        }

        #[test]
        fn scans_arithmetic_operators() {
            let tokens = scan_ok("+ - * /");
            assert_eq!(types(&tokens), vec![Plus, Minus, Star, Slash]);
        }
    }

    mod two_char {
        use super::*;

        #[test]
        fn scans_bang_equal() {
            assert_single("!=", BangEqual);
            assert_lexeme("!=", 0, "!=");
        }

        #[test]
        fn scans_equal_equal() {
            assert_single("==", EqualEqual);
            assert_lexeme("==", 0, "==");
        }

        #[test]
        fn scans_less_equal() {
            assert_single("<=", LessEqual);
        }

        #[test]
        fn scans_greater_equal() {
            assert_single(">=", GreaterEqual);
        }

        #[test]
        fn scans_single_char_when_not_doubled() {
            let tokens = scan_ok("! = < >");
            assert_eq!(types(&tokens), vec![Bang, Equal, Less, Greater]);
        }
    }

    mod literals {
        use super::*;

        #[test]
        fn scans_integer() {
            assert_single("42", Number(42));
            assert_lexeme("42", 0, "42");
        }

        #[test]
        fn scans_large_integer() {
            assert_single("589565", Number(589_565));
        }

        #[test]
        fn scans_float() {
            assert_single("3.15", FloatNumber(3.15));
        }

        #[test]
        fn scans_float_with_integer_part() {
            assert_single("589565.22", FloatNumber(589_565.22));
        }

        #[test]
        fn scans_string() {
            assert_single("\"hello\"", StringToken("hello".into()));
            assert_lexeme("\"hello\"", 0, "hello");
        }

        #[test]
        fn scans_empty_string() {
            assert_single("\"\"", StringToken(String::new()));
        }

        #[test]
        fn scans_string_with_spaces() {
            assert_single("\"hello world\"", StringToken("hello world".into()));
        }

        #[test]
        fn integer_followed_by_comma_stops_at_comma() {
            let tokens = scan_ok("589565,22");
            assert_eq!(types(&tokens), vec![Number(589_565), Comma, Number(22)]);
        }
    }

    mod keywords {
        use super::*;

        #[test]
        fn scans_all_reserved_keywords() {
            let input =
                "and class else false for fun if nil or print return super this true var while";
            let tokens = scan_ok(input);
            assert_eq!(
                types(&tokens),
                vec![
                    And, Class, Else, False, For, Fun, If, Nil, Or, Print, Return, Super, This,
                    True, Var, While,
                ]
            );
        }

        #[test]
        fn keyword_lexeme_matches_text() {
            assert_lexeme("while", 0, "while");
        }
    }

    mod identifiers {
        use super::*;

        #[test]
        fn scans_simple_identifiers() {
            let tokens = scan_ok("my ident");
            assert_eq!(
                types(&tokens),
                vec![Identifier("my".into()), Identifier("ident".into())]
            );
        }

        #[test]
        fn scans_identifier_with_digits() {
            assert_single("foo123", Identifier("foo123".into()));
        }

        #[test]
        fn scans_identifier_with_underscore() {
            assert_single("my_var", Identifier("my_var".into()));
        }

        #[test]
        fn keyword_takes_precedence_over_identifier() {
            assert_single("while", While);
        }
    }

    mod whitespace {
        use super::*;

        #[test]
        fn skips_spaces_tabs_and_carriage_returns() {
            let tokens = scan_ok("  \t\r +");
            assert_eq!(types(&tokens), vec![Plus]);
        }

        #[test]
        fn skips_blank_lines() {
            let tokens = scan_ok("\n\n1\n");
            assert_eq!(types(&tokens), vec![Number(1)]);
        }

        #[test]
        fn empty_input_produces_no_tokens() {
            assert!(scan_ok("").is_empty());
        }
    }

    mod comments {
        use super::*;

        #[test]
        fn skips_line_comment_to_end_of_line() {
            let tokens = scan_ok("foo // comment\nbar");
            assert_eq!(
                types(&tokens),
                vec![Identifier("foo".into()), Identifier("bar".into())]
            );
        }

        #[test]
        fn line_comment_only_line_produces_no_tokens() {
            assert!(scan_ok("// just a comment").is_empty());
        }

        #[test]
        fn skips_block_comment() {
            let tokens = scan_ok("my code /* this is the code\nanother code *\nend */ identifier");
            assert_eq!(
                types(&tokens),
                vec![
                    Identifier("my".into()),
                    Identifier("code".into()),
                    Identifier("identifier".into()),
                ]
            );
        }

        #[test]
        fn inline_block_comment_between_tokens() {
            let tokens = scan_ok("a /* comment */ b");
            assert_eq!(
                types(&tokens),
                vec![Identifier("a".into()), Identifier("b".into())]
            );
        }

        #[test]
        fn star_inside_block_comment_does_not_close_comment() {
            let tokens = scan_ok("a /* * not end */ b");
            assert_eq!(
                types(&tokens),
                vec![Identifier("a".into()), Identifier("b".into())]
            );
        }

        #[test]
        fn slash_inside_block_comment_does_not_close_comment() {
            let tokens = scan_ok("a /* foo / bar */ b");
            assert_eq!(
                types(&tokens),
                vec![Identifier("a".into()), Identifier("b".into())]
            );
        }

        #[test]
        fn tokens_after_block_comment_on_same_line() {
            let tokens = scan_ok("1 /* comment */ + 2");
            assert_eq!(types(&tokens), vec![Number(1), Plus, Number(2)]);
        }

        #[test]
        fn empty_block_comment() {
            let tokens = scan_ok("a /**/ b");
            assert_eq!(
                types(&tokens),
                vec![Identifier("a".into()), Identifier("b".into())]
            );
        }
    }

    mod sequences {
        use super::*;

        #[test]
        fn scans_expression_like_token_stream() {
            let tokens = scan_ok("( 1 + 2 ) * 3 == 4");
            assert_eq!(
                types(&tokens),
                vec![
                    LeftParen,
                    Number(1),
                    Plus,
                    Number(2),
                    RightParen,
                    Star,
                    Number(3),
                    EqualEqual,
                    Number(4),
                ]
            );
        }

        #[test]
        fn scans_mixed_operators_without_whitespace() {
            let tokens = scan_ok("1<=2>=3!=4==5");
            assert_eq!(
                types(&tokens),
                vec![
                    Number(1),
                    LessEqual,
                    Number(2),
                    GreaterEqual,
                    Number(3),
                    BangEqual,
                    Number(4),
                    EqualEqual,
                    Number(5),
                ]
            );
        }

        #[test]
        fn scans_assignment_and_grouping_tokens() {
            let tokens = scan_ok("{ var x = 1; }");
            assert_eq!(
                types(&tokens),
                vec![
                    LeftBrace,
                    Var,
                    Identifier("x".into()),
                    Equal,
                    Number(1),
                    Semicolon,
                    RightBrace,
                ]
            );
        }
    }

    mod line_numbers {
        use super::*;

        #[test]
        fn first_line_is_zero_indexed() {
            assert_line("foo", 0, 0);
        }

        #[test]
        fn tracks_line_after_newline() {
            let input = "foo\nbar";
            assert_line(input, 0, 0);
            assert_line(input, 1, 1);
        }

        #[test]
        fn token_after_comment_on_next_line_has_correct_line() {
            assert_line("// comment\nwhile", 0, 1);
        }
    }

    mod errors {
        use super::*;

        #[test]
        fn rejects_unterminated_string() {
            scan_fails("\"unterminated");
        }

        #[test]
        fn rejects_unterminated_string_across_lines() {
            scan_fails("\"hello\nworld");
        }

        #[test]
        fn rejects_unexpected_character() {
            scan_fails("@");
        }

        #[test]
        fn rejects_invalid_number() {
            scan_fails("1.2.3");
        }

        #[test]
        fn rejects_leading_underscore_identifier() {
            scan_fails("_private");
        }
    }
}
