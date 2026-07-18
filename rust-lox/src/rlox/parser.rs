use crate::rlox::interpreter::RLox;
use crate::rlox::parser::Expr::{Binary, Grouping, Unary};
use crate::rlox::scanner::Scanner;
use crate::rlox::token::TokenType::{
    Bang, BangEqual, Comma, EqualEqual, False, FloatNumber, Greater, GreaterEqual, LeftParen, Less,
    LessEqual, Minus, Nil, Number, Plus, RightParen, Semicolon, Slash, Star, StringToken, True,
};
use crate::rlox::token::{Token, TokenType};

#[derive(Debug)]
enum Literal {
    Number(u64),
    FloatNumber(f64),
    StringL(String),
    True,
    False,
    Nil,
}

#[derive(Debug)]
enum Expr {
    Literal(Literal),
    Binary {
        left: Box<Expr>,
        op: Token,
        right: Box<Expr>,
    },
    Unary {
        op: Token,
        right: Box<Expr>,
    },
    Grouping {
        expr: Box<Expr>,
    },
    Operator {
        token: Token,
    },
}

pub struct Parser<'src, 'lox> {
    scanner: Scanner<'src>,
    current_token: Option<Token>,
    interpreter: &'lox mut RLox,
}

impl<'src, 'lox> Parser<'src, 'lox> {
    pub fn new(scanner: Scanner<'src>, interpreter: &'lox mut RLox) -> Parser<'src, 'lox> {
        Parser {
            scanner,
            current_token: None,
            interpreter,
        }
    }

    pub fn parse(&mut self) {
        while self.expression().is_some() {}
    }

    fn expression(&mut self) -> Option<Expr> {
        let mut exp = self.equality()?;
        while let Some(coma) = self.match_and_return(&Comma) {
            let right = self.equality()?;
            exp = Binary {
                left: Box::new(exp),
                op: coma,
                right: Box::new(right),
            }
        }
        Some(exp)
    }

    fn equality(&mut self) -> Option<Expr> {
        let mut exp = self.comparison()?;

        while let Some(item) = self.match_any_and_return(&[BangEqual, EqualEqual]) {
            let right = self.comparison()?;
            exp = Binary {
                left: Box::new(exp),
                op: item,
                right: Box::new(right),
            }
        }
        Some(exp)
    }

    fn comparison(&mut self) -> Option<Expr> {
        let mut expr = self.term()?;

        while let Some(token) = self.match_any_and_return(&[Greater, GreaterEqual, Less, LessEqual])
        {
            let right = self.term()?;
            expr = Binary {
                left: Box::new(expr),
                op: token,
                right: Box::new(right),
            }
        }
        Some(expr)
    }

    fn term(&mut self) -> Option<Expr> {
        let mut expr = self.factor()?;

        while let Some(token) = self.match_any_and_return(&[Minus, Plus]) {
            let right = self.factor()?;
            expr = Binary {
                left: Box::new(expr),
                op: token,
                right: Box::new(right),
            }
        }
        Some(expr)
    }

    fn factor(&mut self) -> Option<Expr> {
        let mut expr = self.unary()?;

        while let Some(token) = self.match_any_and_return(&[Slash, Star]) {
            let right = self.unary()?;
            expr = Binary {
                left: Box::new(expr),
                op: token,
                right: Box::new(right),
            }
        }
        Some(expr)
    }

    fn unary(&mut self) -> Option<Expr> {
        if let Some(token) = self.match_any_and_return(&[Bang, Minus]) {
            let right = self.unary()?;
            return Some(Unary {
                op: token,
                right: Box::new(right),
            });
        }
        self.primary()
    }

    fn primary(&mut self) -> Option<Expr> {
        let next = self.advance()?;
        match next.token_type.clone() {
            False => Some(Expr::Literal(Literal::False)),
            True => Some(Expr::Literal(Literal::True)),
            Nil => Some(Expr::Literal(Literal::Nil)),
            Number(val) => Some(Expr::Literal(Literal::Number(val))),
            FloatNumber(val) => Some(Expr::Literal(Literal::FloatNumber(val))),
            StringToken(val) => Some(Expr::Literal(Literal::StringL(val.clone()))),
            LeftParen => {
                let expr = self.expression()?;
                self.consume(&RightParen, "Expect ')' after expression.")?;
                Some(Grouping {
                    expr: Box::new(expr),
                })
            }
            _ => {
                self.interpreter
                    .report_token(&next, "Syntax error! Expected expression");
                None
            }
        }
    }

    fn consume(&mut self, token_type: &TokenType, message: &str) -> Option<Token> {
        let next = self.peek().cloned();
        if let Some(next) = next {
            return if next.token_type == *token_type {
                self.advance()
            } else {
                self.interpreter.report_token(&next, message);
                None
            };
        }
        let line = self.scanner.line_index;
        self.interpreter
            .error(line, &format!("Reached EOF, expected {token_type:?}"));
        None
    }
    fn peek(&mut self) -> Option<&Token> {
        if self.current_token.is_none() {
            self.current_token = self.scanner.scan_token(self.interpreter);
        }
        self.current_token.as_ref()
    }

    fn synchronize(&mut self) -> Option<Token> {
        while let Some(token) = self.scanner.scan_token(self.interpreter) {
            if token.token_type == Semicolon {
                if let Some(next) = self.peek() {
                    match next.token_type {
                        TokenType::Class
                        | TokenType::Fun
                        | TokenType::Var
                        | TokenType::For
                        | TokenType::If
                        | TokenType::While
                        | TokenType::Print
                        | TokenType::Return => return self.advance(),
                        _ => {}
                    }
                } else {
                    return None;
                }
            }
        }
        None
    }
    fn advance(&mut self) -> Option<Token> {
        let token = self.current_token.take();
        if token.is_none() {
            self.scanner.scan_token(self.interpreter)
        } else {
            token
        }
    }

    fn check(&mut self, token_type: &TokenType) -> bool {
        if let Some(next) = self.peek() {
            next.token_type == *token_type
        } else {
            false
        }
    }

    fn match_and_return(&mut self, token_type: &TokenType) -> Option<Token> {
        let token = self.peek()?;
        if token.token_type == *token_type {
            return self.advance();
        }
        None
    }

    fn match_any(&mut self, token_types: &[TokenType]) -> bool {
        if let Some(next) = self.peek() {
            token_types.contains(&next.token_type)
        } else {
            false
        }
    }

    fn match_any_and_return(&mut self, token_types: &[TokenType]) -> Option<Token> {
        if let Some(next) = self.peek()
            && token_types.contains(&next.token_type)
        {
            return self.advance();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_expression(input: &str) -> (Option<Expr>, RLox) {
        let mut interpreter = RLox { had_error: false };
        let scanner = Scanner::new(input.lines());
        let mut parser = Parser::new(scanner, &mut interpreter);
        let result = parser.expression();
        (result, interpreter)
    }

    fn parse_ok(input: &str) -> Expr {
        let (result, interpreter) = parse_expression(input);
        assert!(
            !interpreter.had_error,
            "unexpected parse error for {input:?}"
        );
        result.unwrap_or_else(|| panic!("expected expression for {input:?}"))
    }

    fn parse_fails(input: &str) {
        let (result, interpreter) = parse_expression(input);
        assert!(
            interpreter.had_error || result.is_none(),
            "expected parse failure for {input:?}, got {result:?}"
        );
    }

    macro_rules! parsed {
        ($input:expr, |$e:ident| $body:expr) => {{
            let $e = parse_ok($input);
            $body
        }};
    }

    fn as_binary(expr: &Expr) -> (&Expr, TokenType, &Expr) {
        match expr {
            Binary { left, op, right } => (left, op.token_type.clone(), right),
            other => panic!("expected binary expression, got {other:?}"),
        }
    }

    fn as_unary(expr: &Expr) -> (TokenType, &Expr) {
        match expr {
            Unary { op, right } => (op.token_type.clone(), right),
            other => panic!("expected unary expression, got {other:?}"),
        }
    }

    fn as_grouping(expr: &Expr) -> &Expr {
        match expr {
            Grouping { expr } => expr,
            other => panic!("expected grouping expression, got {other:?}"),
        }
    }

    fn as_number(expr: &Expr) -> u64 {
        match expr {
            Expr::Literal(Literal::Number(n)) => *n,
            other => panic!("expected number literal, got {other:?}"),
        }
    }

    fn as_float(expr: &Expr) -> f64 {
        match expr {
            Expr::Literal(Literal::FloatNumber(n)) => *n,
            other => panic!("expected float literal, got {other:?}"),
        }
    }

    fn as_string(expr: &Expr) -> &str {
        match expr {
            Expr::Literal(Literal::StringL(s)) => s.as_str(),
            other => panic!("expected string literal, got {other:?}"),
        }
    }

    mod literals {
        use super::*;

        #[test]
        fn parses_integer() {
            parsed!("42", |expr| {
                assert_eq!(as_number(&expr), 42);
            });
        }

        #[test]
        fn parses_float() {
            parsed!("3.15", |expr| {
                assert!((as_float(&expr) - 3.15).abs() < f64::EPSILON);
            });
        }

        #[test]
        fn parses_string() {
            parsed!("\"hello\"", |expr| {
                assert_eq!(as_string(&expr), "hello");
            });
        }

        #[test]
        fn parses_true() {
            assert!(matches!(parse_ok("true"), Expr::Literal(Literal::True)));
        }

        #[test]
        fn parses_false() {
            assert!(matches!(parse_ok("false"), Expr::Literal(Literal::False)));
        }

        #[test]
        fn parses_nil() {
            assert!(matches!(parse_ok("nil"), Expr::Literal(Literal::Nil)));
        }
    }

    mod binary_operators {
        use super::*;

        #[test]
        fn parses_addition() {
            parsed!("1 + 2", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Plus);
                assert_eq!(as_number(left), 1);
                assert_eq!(as_number(right), 2);
            });
        }

        #[test]
        fn parses_subtraction() {
            parsed!("5 - 3", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, Minus);
            });
        }

        #[test]
        fn parses_multiplication() {
            parsed!("3 * 4", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, Star);
            });
        }

        #[test]
        fn parses_division() {
            parsed!("8 / 2", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, Slash);
            });
        }

        #[test]
        fn parses_equality() {
            parsed!("1 == 2", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
            });
        }

        #[test]
        fn parses_inequality() {
            parsed!("1 != 2", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, BangEqual);
            });
        }

        #[test]
        fn parses_less_than() {
            parsed!("1 < 2", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, Less);
            });
        }

        #[test]
        fn parses_less_equal() {
            parsed!("1 <= 2", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, LessEqual);
            });
        }

        #[test]
        fn parses_greater_than() {
            parsed!("2 > 1", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, Greater);
            });
        }

        #[test]
        fn parses_greater_equal() {
            parsed!("2 >= 1", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, GreaterEqual);
            });
        }

        #[test]
        fn parses_chained_addition_left_associative() {
            parsed!("1 + 2 + 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Plus);
                assert_eq!(as_number(right), 3);
                let (inner_left, inner_op, inner_right) = as_binary(left);
                assert_eq!(inner_op, Plus);
                assert_eq!(as_number(inner_left), 1);
                assert_eq!(as_number(inner_right), 2);
            });
        }

        #[test]
        fn parses_chained_equality_left_associative() {
            parsed!("1 == 2 == 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
                assert_eq!(as_number(right), 3);
                let (_, inner_op, _) = as_binary(left);
                assert_eq!(inner_op, EqualEqual);
            });
        }
    }

    mod precedence {
        use super::*;

        #[test]
        fn multiplication_binds_tighter_than_addition() {
            parsed!("1 + 2 * 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Plus);
                assert_eq!(as_number(left), 1);
                let (_, mul_op, _) = as_binary(right);
                assert_eq!(mul_op, Star);
            });
        }

        #[test]
        fn addition_binds_looser_than_multiplication() {
            parsed!("1 * 2 + 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Plus);
                assert_eq!(as_number(right), 3);
                let (_, mul_op, _) = as_binary(left);
                assert_eq!(mul_op, Star);
            });
        }

        #[test]
        fn addition_binds_tighter_than_equality() {
            parsed!("1 + 2 == 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
                assert_eq!(as_number(right), 3);
                let (_, add_op, _) = as_binary(left);
                assert_eq!(add_op, Plus);
            });
        }

        #[test]
        fn comparison_binds_tighter_than_equality() {
            parsed!("1 < 2 == 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
                assert_eq!(as_number(right), 3);
                let (_, cmp_op, _) = as_binary(left);
                assert_eq!(cmp_op, Less);
            });
        }

        #[test]
        fn term_binds_tighter_than_comparison() {
            parsed!("1 + 2 < 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Less);
                assert_eq!(as_number(right), 3);
                let (_, add_op, _) = as_binary(left);
                assert_eq!(add_op, Plus);
            });
        }
    }

    mod unary {
        use super::*;

        #[test]
        fn parses_negation() {
            parsed!("-1", |expr| {
                let (op, right) = as_unary(&expr);
                assert_eq!(op, Minus);
                assert_eq!(as_number(right), 1);
            });
        }

        #[test]
        fn parses_logical_not() {
            parsed!("!true", |expr| {
                let (op, right) = as_unary(&expr);
                assert_eq!(op, Bang);
                assert!(matches!(right, Expr::Literal(Literal::True)));
            });
        }

        #[test]
        fn parses_nested_unary_right_associative() {
            parsed!("--1", |expr| {
                let (outer_op, inner) = as_unary(&expr);
                assert_eq!(outer_op, Minus);
                let (inner_op, right) = as_unary(inner);
                assert_eq!(inner_op, Minus);
                assert_eq!(as_number(right), 1);
            });
        }

        #[test]
        fn unary_binds_tighter_than_multiplication() {
            parsed!("-1 * 2", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Star);
                let (unary_op, _) = as_unary(left);
                assert_eq!(unary_op, Minus);
                assert_eq!(as_number(right), 2);
            });
        }

        #[test]
        fn unary_binds_tighter_than_equality() {
            parsed!("!true == false", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
                let (bang_op, _) = as_unary(left);
                assert_eq!(bang_op, Bang);
                assert!(matches!(right, Expr::Literal(Literal::False)));
            });
        }
    }

    mod grouping {
        use super::*;

        #[test]
        fn parses_parenthesized_expression() {
            parsed!("(1 + 2)", |expr| {
                let inner = as_grouping(&expr);
                let (_, op, _) = as_binary(inner);
                assert_eq!(op, Plus);
            });
        }

        #[test]
        fn parentheses_override_precedence() {
            parsed!("(1 + 2) * 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Star);
                assert_eq!(as_number(right), 3);
                let inner = as_grouping(left);
                let (_, add_op, _) = as_binary(inner);
                assert_eq!(add_op, Plus);
            });
        }

        #[test]
        fn parses_nested_parentheses() {
            parsed!("((42))", |expr| {
                let inner = as_grouping(&expr);
                let deeper = as_grouping(inner);
                assert_eq!(as_number(deeper), 42);
            });
        }

        #[test]
        fn grouping_can_wrap_comma_expression() {
            parsed!("(1, 2)", |expr| {
                let inner = as_grouping(&expr);
                let (_, op, _) = as_binary(inner);
                assert_eq!(op, Comma);
            });
        }
    }

    mod comma {
        use super::*;

        #[test]
        fn parses_comma_with_both_operands() {
            parsed!("1 + 2 , 2 + 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Comma);

                let (left_left, left_op, left_right) = as_binary(left);
                assert_eq!(left_op, Plus);
                assert_eq!(as_number(left_left), 1);
                assert_eq!(as_number(left_right), 2);

                let (right_left, right_op, right_right) = as_binary(right);
                assert_eq!(right_op, Plus);
                assert_eq!(as_number(right_left), 2);
                assert_eq!(as_number(right_right), 3);
            });
        }

        #[test]
        fn comma_is_left_associative() {
            parsed!("1, 2, 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Comma);
                assert_eq!(as_number(right), 3);

                let (inner_left, inner_op, inner_right) = as_binary(left);
                assert_eq!(inner_op, Comma);
                assert_eq!(as_number(inner_left), 1);
                assert_eq!(as_number(inner_right), 2);
            });
        }

        #[test]
        fn comma_has_lower_precedence_than_equality() {
            parsed!("1 == 2, 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Comma);
                assert_eq!(as_number(right), 3);
                let (_, eq_op, _) = as_binary(left);
                assert_eq!(eq_op, EqualEqual);
            });
        }

        #[test]
        fn comma_has_lower_precedence_than_addition() {
            parsed!("1 + 2, 3 + 4", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Comma);
                let (_, left_op, _) = as_binary(left);
                assert_eq!(left_op, Plus);
                let (_, right_op, _) = as_binary(right);
                assert_eq!(right_op, Plus);
            });
        }
    }

    mod complex {
        use super::*;

        #[test]
        fn parses_mixed_expression() {
            parsed!("!(1 + 2) * -3 < 10 == true", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
                assert!(matches!(right, Expr::Literal(Literal::True)));
                let (_, cmp_op, _) = as_binary(left);
                assert_eq!(cmp_op, Less);
            });
        }

        #[test]
        fn parses_expression_with_string_and_nil() {
            parsed!("\"a\" == nil", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
                assert_eq!(as_string(left), "a");
                assert!(matches!(right, Expr::Literal(Literal::Nil)));
            });
        }

        #[test]
        fn parses_float_in_arithmetic() {
            parsed!("1.5 + 2", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Plus);
                assert!((as_float(left) - 1.5).abs() < f64::EPSILON);
                assert_eq!(as_number(right), 2);
            });
        }
    }

    mod errors {
        use super::*;

        #[test]
        fn rejects_unclosed_parenthesis() {
            parse_fails("((1 + 2)");
        }

        #[test]
        fn rejects_extra_closing_parenthesis() {
            let mut interpreter = RLox { had_error: false };
            let scanner = Scanner::new("(1 + 2))".lines());
            let mut parser = Parser::new(scanner, &mut interpreter);
            parser.parse();
            assert!(interpreter.had_error);
        }

        #[test]
        fn rejects_empty_parentheses() {
            parse_fails("()");
        }

        #[test]
        fn rejects_bare_operator_at_start() {
            parse_fails("+ 1");
        }

        #[test]
        fn rejects_missing_right_operand_after_plus() {
            parse_fails("1 +");
        }

        #[test]
        fn rejects_missing_right_operand_after_equality() {
            parse_fails("1 ==");
        }

        #[test]
        fn rejects_missing_right_operand_after_comma() {
            parse_fails("1,");
        }

        #[test]
        fn rejects_unary_without_operand() {
            parse_fails("!");
        }

        #[test]
        fn rejects_identifier() {
            parse_fails("foo");
        }

        #[test]
        fn rejects_unexpected_token() {
            parse_fails(")");
        }

        #[test]
        fn rejects_empty_input() {
            parse_fails("");
        }
    }
}
