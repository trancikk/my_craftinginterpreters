use crate::rlox::interpreter::RloxError;
use crate::rlox::parser::Expr::{Binary, Grouping, MissingLeftOperand, Ternary, Unary};
use crate::rlox::scanner::Scanner;
use crate::rlox::token::TokenType::{
    Bang, BangEqual, Colon, Comma, EOF, EqualEqual, False, FloatNumber, Greater, GreaterEqual,
    LeftParen, Less, LessEqual, Minus, Nil, Number, Plus, Question, RightParen, Semicolon, Slash,
    Star, StringToken, True,
};
use crate::rlox::token::{Token, TokenType};

#[derive(Debug)]
pub enum Literal {
    Number(u64),
    FloatNumber(f64),
    StringL(String),
    True,
    False,
    Nil,
}

#[derive(Debug)]
pub enum Expr {
    SyntaxLiteral(Literal),
    Binary {
        left: Box<Expr>,
        op: Token,
        right: Box<Expr>,
    },
    Ternary {
        predicate: Box<Expr>,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: Token,
        right: Box<Expr>,
    },
    Grouping {
        expr: Box<Expr>,
    },
    MissingLeftOperand {
        token: Token,
    },
}

pub struct Parser<'src> {
    scanner: Scanner<'src>,
    current_token: Option<Token>,
}

impl<'src> Parser<'src> {
    pub fn new(scanner: Scanner<'src>) -> Parser<'src> {
        Parser {
            scanner,
            current_token: None,
        }
    }

    pub fn parse(&mut self) {
        while self.expression().is_ok() {}
    }

    fn report_binary_operation_error(token: Token) -> RloxError {
        let msg = format!("Binary operation {token:?} is missing left argument!");
        RloxError::ParserError(token, msg)
    }

    fn produce_error_for_expression(token: Token, msg: &str) -> Result<Expr, RloxError> {
        Err(RloxError::ParserError(token, String::from(msg)))
    }

    fn produce_error_for_token(token: Token, msg: &str) -> Result<Token, RloxError> {
        Err(RloxError::ParserError(token, String::from(msg)))
    }

    fn produce_and_report_binary_error(expr: Expr, token: Token) -> Result<Expr, RloxError> {
        Ok(Binary {
            left: Box::new(MissingLeftOperand {
                token: token.clone(),
            }),
            op: token,
            right: Box::new(expr),
        })
    }

    // fn produce_and_report_binary_error(expr: Expr, token: Token) -> Result<Expr, RloxError> {
    //     drop(expr);
    //     // self.produce_binary_error(expr, token);
    //     Err(Self::report_binary_operation_error(token))
    // }

    fn loop_inside_same_level(
        &mut self,
        ops: &[TokenType],
        next_level: fn(&mut Self) -> Result<Expr, RloxError>,
    ) -> Result<Expr, RloxError> {
        let mut expr = next_level(self)?;
        loop {
            let token = self.peek()?;
            if token.token_type == EOF {
                break;
            } else if ops.contains(&token.token_type) {
                let token = self.advance()?;
                let right = next_level(self)?;
                expr = Binary {
                    left: Box::new(expr),
                    op: token,
                    right: Box::new(right),
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }

    pub fn expression(&mut self) -> Result<Expr, RloxError> {
        if let Ok(token) = self.match_and_return(&Comma) {
            let expr = self.expression()?;
            return Self::produce_and_report_binary_error(expr, token);
        }
        self.loop_inside_same_level(&[Comma], Self::ternary)
    }

    fn ternary(&mut self) -> Result<Expr, RloxError> {
        let mut expr = self.equality()?;
        if self.match_and_return(&Question).is_ok() {
            let left = self.expression()?;
            self.consume(
                &Colon,
                "Expected ':' in a ternary operator after the predicate",
            )?;
            let ternary_result = self.ternary()?;
            expr = Ternary {
                predicate: Box::new(expr),
                left: Box::new(left),
                right: Box::new(ternary_result),
            }
        }
        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr, RloxError> {
        let ops = [BangEqual, EqualEqual];
        if let Ok(token) = self.match_any_and_return(&ops) {
            let expr = self.equality()?;
            return Self::produce_and_report_binary_error(expr, token);
        }
        self.loop_inside_same_level(&ops, Self::comparison)
    }

    fn comparison(&mut self) -> Result<Expr, RloxError> {
        let ops = [Greater, GreaterEqual, Less, LessEqual];
        if let Ok(token) = self.match_any_and_return(&ops) {
            let expr = self.comparison()?;
            return Self::produce_and_report_binary_error(expr, token);
        }

        self.loop_inside_same_level(&ops, Self::term)
    }

    fn term(&mut self) -> Result<Expr, RloxError> {
        if let Ok(token) = self.match_and_return(&Plus) {
            let expr = self.term()?;
            return Self::produce_and_report_binary_error(expr, token);
        }

        self.loop_inside_same_level(&[Minus, Plus], Self::factor)
    }

    fn factor(&mut self) -> Result<Expr, RloxError> {
        let ops = [Slash, Star];

        if let Ok(token) = self.match_any_and_return(&ops) {
            let expr = self.factor()?;
            return Self::produce_and_report_binary_error(expr, token);
        }

        self.loop_inside_same_level(&ops, Self::unary)
    }

    fn unary(&mut self) -> Result<Expr, RloxError> {
        if let Ok(token) = self.match_any_and_return(&[Bang, Minus]) {
            let right = self.unary()?;
            return Ok(Unary {
                op: token,
                right: Box::new(right),
            });
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, RloxError> {
        let next = self.advance()?;
        match next.token_type.clone() {
            False => Ok(Expr::SyntaxLiteral(Literal::False)),
            True => Ok(Expr::SyntaxLiteral(Literal::True)),
            Nil => Ok(Expr::SyntaxLiteral(Literal::Nil)),
            Number(val) => Ok(Expr::SyntaxLiteral(Literal::Number(val))),
            FloatNumber(val) => Ok(Expr::SyntaxLiteral(Literal::FloatNumber(val))),
            StringToken(val) => Ok(Expr::SyntaxLiteral(Literal::StringL(val.clone()))),
            LeftParen => {
                let expr = self.expression()?;
                self.consume(&RightParen, "Expect ')' after expression.")?;
                Ok(Grouping {
                    expr: Box::new(expr),
                })
            }
            EOF => Err(RloxError::EOF),
            _ => {
                Self::produce_error_for_expression(next, "Syntax error! Expected expression")
                // self.interpreter
                //     .report_token(&next, );
                // None
            }
        }
    }

    fn consume(&mut self, token_type: &TokenType, message: &str) -> Result<Token, RloxError> {
        let next = self.peek().cloned()?;
        if next.token_type == *token_type {
            self.advance()
        } else {
            Self::produce_error_for_token(next, message)
        }
    }

    fn peek(&mut self) -> Result<&Token, RloxError> {
        if self.current_token.is_none() {
            let next_token = self.scanner.scan_token()?;
            self.current_token = Some(next_token);
        }
        Ok(self.current_token.as_ref().unwrap())
    }

    // TODO maybe return just token or EOF
    fn synchronize(&mut self) -> Result<Token, RloxError> {
        loop {
            let token = self.scanner.scan_token()?;
            if token.token_type == Semicolon {
                let next = self.peek()?;
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
            } else if token.token_type == TokenType::EOF {
                return self.advance();
            }
        }
    }
    fn advance(&mut self) -> Result<Token, RloxError> {
        let token = self.current_token.take();
        if let Some(token) = token {
            Ok(token)
        } else {
            self.scanner.scan_token()
        }
    }

    fn check(&mut self, token_type: &TokenType) -> bool {
        if let Ok(next) = self.peek() {
            next.token_type == *token_type
        } else {
            false
        }
    }

    fn match_and_return(&mut self, token_type: &TokenType) -> Result<Token, RloxError> {
        let token = self.peek().cloned()?;
        if token.token_type == *token_type {
            return self.advance();
        }
        let msg = format!("Expected token: {token_type:?}, got: {token:?}");
        Err(RloxError::ParserError(token, msg))
    }

    fn match_any(&mut self, token_types: &[TokenType]) -> bool {
        if let Ok(next) = self.peek() {
            token_types.contains(&next.token_type)
        } else {
            false
        }
    }

    fn match_any_and_return(&mut self, token_types: &[TokenType]) -> Result<Token, RloxError> {
        let next = self.peek().cloned()?;
        if token_types.contains(&next.token_type) {
            return self.advance();
        }
        let msg = format!("Expected tokens: {token_types:?}, got: {next:?}");
        Err(RloxError::ParserError(next, msg))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[cfg(test)]
    pub(crate) fn parse_one_expression(input: &str) -> Expr {
        use crate::rlox::scanner::Scanner;

        let scanner = Scanner::new(input.lines());
        let mut parser = Parser::new(scanner);
        let result = parser.expression();
        assert!(result.is_ok());
        result.unwrap_or_else(|_| panic!("expected expression for {input:?}"))
    }

    pub(crate) fn parse_expression(input: &str) -> Result<Expr, RloxError> {
        let scanner = Scanner::new(input.lines());
        let mut parser = Parser::new(scanner);
        parser.expression()
    }

    pub(crate) fn parse_ok(input: &str) -> Expr {
        let result = parse_expression(input);
        result.unwrap_or_else(|_| panic!("expected expression for {input:?}"))
    }

    pub(crate) fn parse_fails(input: &str) {
        let result = parse_expression(input);
        assert!(
            result.is_err(),
            "expected parse failure for {input:?}, got {result:?}"
        );
    }

    macro_rules! parsed {
        ($input:expr, |$e:ident| $body:expr) => {{
            let $e = parse_ok($input);
            $body
        }};
    }

    pub(crate) fn as_binary(expr: &Expr) -> (&Expr, TokenType, &Expr) {
        match expr {
            Binary { left, op, right } => (left, op.token_type.clone(), right),
            other => panic!("expected binary expression, got {other:?}"),
        }
    }

    pub(crate) fn as_unary(expr: &Expr) -> (TokenType, &Expr) {
        match expr {
            Unary { op, right } => (op.token_type.clone(), right),
            other => panic!("expected unary expression, got {other:?}"),
        }
    }

    pub(crate) fn as_grouping(expr: &Expr) -> &Expr {
        match expr {
            Grouping { expr } => expr,
            other => panic!("expected grouping expression, got {other:?}"),
        }
    }

    pub(crate) fn as_number(expr: &Expr) -> u64 {
        match expr {
            Expr::SyntaxLiteral(Literal::Number(n)) => *n,
            other => panic!("expected number literal, got {other:?}"),
        }
    }

    pub(crate) fn as_float(expr: &Expr) -> f64 {
        match expr {
            Expr::SyntaxLiteral(Literal::FloatNumber(n)) => *n,
            other => panic!("expected float literal, got {other:?}"),
        }
    }

    pub(crate) fn as_string(expr: &Expr) -> &str {
        match expr {
            Expr::SyntaxLiteral(Literal::StringL(s)) => s.as_str(),
            other => panic!("expected string literal, got {other:?}"),
        }
    }

    pub(crate) fn as_ternary(expr: &Expr) -> (&Expr, &Expr, &Expr) {
        match expr {
            Expr::Ternary {
                predicate: condition,
                left,
                right,
            } => (condition, left, right),
            other => panic!("expected ternary expression, got {other:?}"),
        }
    }

    pub(crate) fn parse_with_error(input: &str) -> Result<Expr, RloxError> {
        parse_expression(input)
    }

    pub(crate) fn parse_error_recovery(input: &str) -> Expr {
        let result = parse_with_error(input);
        result.unwrap_or_else(|_| panic!("expected recovered expression for {input:?}"))
    }

    pub(crate) fn as_operator(expr: &Expr) -> TokenType {
        match expr {
            Expr::MissingLeftOperand { token } => token.token_type.clone(),
            other => panic!("expected operator placeholder, got {other:?}"),
        }
    }

    pub(crate) fn assert_missing_left_binary(
        expr: &Expr,
        expected_op: TokenType,
    ) -> (&Expr, &Expr) {
        let (left, op, right) = as_binary(expr);
        assert_eq!(op, expected_op);
        assert_eq!(as_operator(left), expected_op);
        (left, right)
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
            assert!(matches!(
                parse_ok("true"),
                Expr::SyntaxLiteral(Literal::True)
            ));
        }

        #[test]
        fn parses_false() {
            assert!(matches!(
                parse_ok("false"),
                Expr::SyntaxLiteral(Literal::False)
            ));
        }

        #[test]
        fn parses_nil() {
            assert!(matches!(parse_ok("nil"), Expr::SyntaxLiteral(Literal::Nil)));
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
                assert!(matches!(right, Expr::SyntaxLiteral(Literal::True)));
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
                assert!(matches!(right, Expr::SyntaxLiteral(Literal::False)));
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

    mod ternary {
        use super::*;

        #[test]
        fn parses_simple_ternary() {
            parsed!("true ? 1 : 2", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                assert!(matches!(cond, Expr::SyntaxLiteral(Literal::True)));
                assert_eq!(as_number(then_branch), 1);
                assert_eq!(as_number(else_branch), 2);
            });
        }

        #[test]
        fn parses_ternary_with_false_condition() {
            parsed!("false ? 1 : 2", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                assert!(matches!(cond, Expr::SyntaxLiteral(Literal::False)));
                assert_eq!(as_number(then_branch), 1);
                assert_eq!(as_number(else_branch), 2);
            });
        }

        #[test]
        fn parses_ternary_with_nil_and_string_branches() {
            parsed!("nil ? \"yes\" : \"no\"", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                assert!(matches!(cond, Expr::SyntaxLiteral(Literal::Nil)));
                assert_eq!(as_string(then_branch), "yes");
                assert_eq!(as_string(else_branch), "no");
            });
        }

        #[test]
        fn chained_ternary_is_right_associative() {
            parsed!("1 ? 2 : 3 ? 4 : 5", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                assert_eq!(as_number(cond), 1);
                assert_eq!(as_number(then_branch), 2);

                let (inner_cond, inner_then, inner_else) = as_ternary(else_branch);
                assert_eq!(as_number(inner_cond), 3);
                assert_eq!(as_number(inner_then), 4);
                assert_eq!(as_number(inner_else), 5);
            });
        }

        #[test]
        fn nested_ternary_in_then_branch() {
            parsed!("1 ? 2 ? 3 : 4 : 5", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                assert_eq!(as_number(cond), 1);
                assert_eq!(as_number(else_branch), 5);

                let (inner_cond, inner_then, inner_else) = as_ternary(then_branch);
                assert_eq!(as_number(inner_cond), 2);
                assert_eq!(as_number(inner_then), 3);
                assert_eq!(as_number(inner_else), 4);
            });
        }

        #[test]
        fn nested_ternary_in_else_branch_via_equality_precedence() {
            parsed!("true ? 1 : 2 == 3 ? 4 : 5", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                assert!(matches!(cond, Expr::SyntaxLiteral(Literal::True)));
                assert_eq!(as_number(then_branch), 1);

                let (inner_cond, inner_then, inner_else) = as_ternary(else_branch);
                let (_, eq_op, _) = as_binary(inner_cond);
                assert_eq!(eq_op, EqualEqual);
                assert_eq!(as_number(inner_then), 4);
                assert_eq!(as_number(inner_else), 5);
            });
        }

        #[test]
        fn arithmetic_in_condition_is_fully_parsed() {
            parsed!("1 + 2 ? 3 : 4", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                let (_, add_op, _) = as_binary(cond);
                assert_eq!(add_op, Plus);
                assert_eq!(as_number(then_branch), 3);
                assert_eq!(as_number(else_branch), 4);
            });
        }

        #[test]
        fn equality_in_condition_is_fully_parsed() {
            parsed!("1 == 2 ? 3 : 4", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                let (_, eq_op, _) = as_binary(cond);
                assert_eq!(eq_op, EqualEqual);
                assert_eq!(as_number(then_branch), 3);
                assert_eq!(as_number(else_branch), 4);
            });
        }

        #[test]
        fn comparison_in_condition_is_fully_parsed() {
            parsed!("1 < 2 ? 3 : 4", |expr| {
                let (cond, _, _) = as_ternary(&expr);
                let (_, cmp_op, _) = as_binary(cond);
                assert_eq!(cmp_op, Less);
            });
        }

        #[test]
        fn unary_in_condition_binds_tighter_than_ternary() {
            parsed!("!true ? 1 : 2", |expr| {
                let (cond, _, _) = as_ternary(&expr);
                let (bang_op, _) = as_unary(cond);
                assert_eq!(bang_op, Bang);
            });
        }

        #[test]
        fn negation_in_condition_binds_tighter_than_ternary() {
            parsed!("-1 ? 2 : 3", |expr| {
                let (cond, _, _) = as_ternary(&expr);
                let (minus_op, inner) = as_unary(cond);
                assert_eq!(minus_op, Minus);
                assert_eq!(as_number(inner), 1);
            });
        }

        #[test]
        fn else_branch_parses_equality_before_next_ternary() {
            parsed!("true ? 1 : 2 == 3", |expr| {
                let (_, _, else_branch) = as_ternary(&expr);
                let (_, eq_op, _) = as_binary(else_branch);
                assert_eq!(eq_op, EqualEqual);
            });
        }

        #[test]
        fn arithmetic_in_branches_is_parsed() {
            parsed!("true ? 1 + 2 : 3 * 4", |expr| {
                let (_, then_branch, else_branch) = as_ternary(&expr);
                let (_, then_op, _) = as_binary(then_branch);
                assert_eq!(then_op, Plus);
                let (_, else_op, _) = as_binary(else_branch);
                assert_eq!(else_op, Star);
            });
        }

        #[test]
        fn parentheses_override_ternary_precedence() {
            parsed!("(1 ? 2 : 3) + 4", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Plus);
                assert_eq!(as_number(right), 4);
                let inner = as_grouping(left);
                let (cond, then_branch, else_branch) = as_ternary(inner);
                assert_eq!(as_number(cond), 1);
                assert_eq!(as_number(then_branch), 2);
                assert_eq!(as_number(else_branch), 3);
            });
        }

        #[test]
        fn parentheses_force_left_associative_chaining() {
            parsed!("(1 ? 2 : 3) ? 4 : 5", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                let inner = as_grouping(cond);
                let (inner_cond, inner_then, inner_else) = as_ternary(inner);
                assert_eq!(as_number(inner_cond), 1);
                assert_eq!(as_number(inner_then), 2);
                assert_eq!(as_number(inner_else), 3);
                assert_eq!(as_number(then_branch), 4);
                assert_eq!(as_number(else_branch), 5);
            });
        }

        #[test]
        fn grouping_can_wrap_entire_ternary() {
            parsed!("(true ? 1 : 2)", |expr| {
                let inner = as_grouping(&expr);
                let (cond, then_branch, else_branch) = as_ternary(inner);
                assert!(matches!(cond, Expr::SyntaxLiteral(Literal::True)));
                assert_eq!(as_number(then_branch), 1);
                assert_eq!(as_number(else_branch), 2);
            });
        }

        #[test]
        fn ternary_binds_tighter_than_comma_on_the_right() {
            parsed!("1 ? 2 : 3, 4", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Comma);
                assert_eq!(as_number(right), 4);
                let (_, then_branch, else_branch) = as_ternary(left);
                assert_eq!(as_number(then_branch), 2);
                assert_eq!(as_number(else_branch), 3);
            });
        }

        #[test]
        fn ternary_binds_tighter_than_comma_on_the_left() {
            parsed!("1, 2 ? 3 : 4", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Comma);
                assert_eq!(as_number(left), 1);
                let (_, then_branch, else_branch) = as_ternary(right);
                assert_eq!(as_number(then_branch), 3);
                assert_eq!(as_number(else_branch), 4);
            });
        }

        #[test]
        fn then_branch_allows_comma_expression() {
            parsed!("true ? 1, 2 : 3", |expr| {
                let (_, then_branch, else_branch) = as_ternary(&expr);
                let (_, comma_op, _) = as_binary(then_branch);
                assert_eq!(comma_op, Comma);
                assert_eq!(as_number(else_branch), 3);
            });
        }

        #[test]
        fn rejects_missing_then_branch() {
            parse_fails("1 ?");
        }

        #[test]
        fn rejects_missing_colon() {
            parse_fails("1 ? 2");
        }

        #[test]
        fn rejects_missing_else_branch() {
            parse_fails("1 ? 2 :");
        }

        #[test]
        fn rejects_leading_question_mark() {
            parse_fails("? 1 : 2");
        }

        #[test]
        fn rejects_bare_question_mark() {
            parse_fails("?");
        }

        #[test]
        fn rejects_double_question_without_colon() {
            parse_fails("1 ? ? 2 : 3");
        }

        #[test]
        fn rejects_ternary_without_condition() {
            parse_fails("? true : false");
        }
    }

    mod complex {
        use super::*;

        #[test]
        fn parses_mixed_expression() {
            parsed!("!(1 + 2) * -3 < 10 == true", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
                assert!(matches!(right, Expr::SyntaxLiteral(Literal::True)));
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
                assert!(matches!(right, Expr::SyntaxLiteral(Literal::Nil)));
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
            let scanner = Scanner::new("(1 + 2))".lines());
            let mut parser = Parser::new(scanner);
            let result = parser.expression();
            let result = parser.expression();
            assert!(result.is_err());
        }

        #[test]
        fn rejects_empty_parentheses() {
            parse_fails("()");
        }

        #[test]
        fn rejects_bare_operator_at_start() {
            parse_error_recovery("+ 1");
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

    mod missing_left_operand {
        use super::*;

        mod term {
            use super::*;

            #[test]
            fn recovers_bare_plus_with_number() {
                let expr = parse_error_recovery("+ 1");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                assert_eq!(as_number(right), 1);
            }

            #[test]
            fn recovers_bare_plus_with_float() {
                let expr = parse_error_recovery("+ 1.5");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                assert!((as_float(right) - 1.5).abs() < f64::EPSILON);
            }

            #[test]
            fn recovers_bare_plus_with_string() {
                let expr = parse_error_recovery("+ \"hi\"");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                assert_eq!(as_string(right), "hi");
            }

            #[test]
            fn rhs_parses_full_product_for_plus() {
                let expr = parse_error_recovery("+ 1 * 2");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                let (left, op, right) = as_binary(right);
                assert_eq!(op, Star);
                assert_eq!(as_number(left), 1);
                assert_eq!(as_number(right), 2);
            }

            #[test]
            fn rhs_parses_chained_addition() {
                let expr = parse_error_recovery("+ 1 + 2");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                let (inner_left, inner_op, inner_right) = as_binary(right);
                assert_eq!(inner_op, Plus);
                assert_eq!(as_number(inner_left), 1);
                assert_eq!(as_number(inner_right), 2);
            }

            #[test]
            fn chained_missing_plus_operators() {
                let expr = parse_error_recovery("+ + 1");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                let (_, inner_op, inner_right) = as_binary(right);
                assert_eq!(inner_op, Plus);
                assert_eq!(as_number(inner_right), 1);
            }

            #[test]
            fn recovers_plus_with_unary_negation_rhs() {
                let expr = parse_error_recovery("+ -1");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                let (unary_op, inner) = as_unary(right);
                assert_eq!(unary_op, Minus);
                assert_eq!(as_number(inner), 1);
            }

            #[test]
            fn recovers_inside_parentheses() {
                let expr = parse_error_recovery("(+ 1)");
                let inner = as_grouping(&expr);
                let (_, right) = assert_missing_left_binary(inner, Plus);
                assert_eq!(as_number(right), 1);
            }

            #[test]
            fn unary_minus_is_not_missing_left_operand() {
                parsed!("-1", |expr| {
                    let (op, right) = as_unary(&expr);
                    assert_eq!(op, Minus);
                    assert_eq!(as_number(right), 1);
                });
            }

            #[test]
            fn spaced_unary_minus_is_not_missing_left_operand() {
                parsed!("- 1", |expr| {
                    let (op, right) = as_unary(&expr);
                    assert_eq!(op, Minus);
                    assert_eq!(as_number(right), 1);
                });
            }
        }

        mod factor {
            use super::*;

            #[test]
            fn recovers_bare_star_with_number() {
                let expr = parse_error_recovery("* 2");
                let (_, right) = assert_missing_left_binary(&expr, Star);
                assert_eq!(as_number(right), 2);
            }

            #[test]
            fn recovers_bare_slash_with_number() {
                let expr = parse_error_recovery("/ 4");
                let (_, right) = assert_missing_left_binary(&expr, Slash);
                assert_eq!(as_number(right), 4);
            }

            #[test]
            fn rhs_parses_full_quotient() {
                let expr = parse_error_recovery("* 6 / 2");
                let (_, right) = assert_missing_left_binary(&expr, Star);
                let (left, op, right) = as_binary(right);
                assert_eq!(op, Slash);
                assert_eq!(as_number(left), 6);
                assert_eq!(as_number(right), 2);
            }

            #[test]
            fn rhs_parses_unary_operand() {
                let expr = parse_error_recovery("* -3");
                let (_, right) = assert_missing_left_binary(&expr, Star);
                let (unary_op, inner) = as_unary(right);
                assert_eq!(unary_op, Minus);
                assert_eq!(as_number(inner), 3);
            }

            #[test]
            fn chained_missing_star_operators() {
                let expr = parse_error_recovery("* * 2");
                let (_, right) = assert_missing_left_binary(&expr, Star);
                let (_, inner_op, inner_right) = as_binary(right);
                assert_eq!(inner_op, Star);
                assert_eq!(as_number(inner_right), 2);
            }
        }

        mod comparison {
            use super::*;

            #[test]
            fn recovers_bare_less_with_number() {
                let expr = parse_error_recovery("< 3");
                let (_, right) = assert_missing_left_binary(&expr, Less);
                assert_eq!(as_number(right), 3);
            }

            #[test]
            fn recovers_bare_less_equal_with_number() {
                let expr = parse_error_recovery("<= 3");
                let (_, right) = assert_missing_left_binary(&expr, LessEqual);
                assert_eq!(as_number(right), 3);
            }

            #[test]
            fn recovers_bare_greater_with_number() {
                let expr = parse_error_recovery("> 5");
                let (_, right) = assert_missing_left_binary(&expr, Greater);
                assert_eq!(as_number(right), 5);
            }

            #[test]
            fn recovers_bare_greater_equal_with_number() {
                let expr = parse_error_recovery(">= 5");
                let (_, right) = assert_missing_left_binary(&expr, GreaterEqual);
                assert_eq!(as_number(right), 5);
            }

            #[test]
            fn rhs_parses_addition_within_comparison() {
                let expr = parse_error_recovery("< 1 + 2");
                let (_, right) = assert_missing_left_binary(&expr, Less);
                let (left, op, right) = as_binary(right);
                assert_eq!(op, Plus);
                assert_eq!(as_number(left), 1);
                assert_eq!(as_number(right), 2);
            }
        }

        mod equality {
            use super::*;

            #[test]
            fn recovers_bare_equal_equal_with_number() {
                let expr = parse_error_recovery("== 1");
                let (_, right) = assert_missing_left_binary(&expr, EqualEqual);
                assert_eq!(as_number(right), 1);
            }

            #[test]
            fn recovers_bare_bang_equal_with_number() {
                let expr = parse_error_recovery("!= 2");
                let (_, right) = assert_missing_left_binary(&expr, BangEqual);
                assert_eq!(as_number(right), 2);
            }

            #[test]
            fn rhs_parses_comparison_within_equality() {
                let expr = parse_error_recovery("== 1 < 2");
                let (_, right) = assert_missing_left_binary(&expr, EqualEqual);
                let (_, cmp_op, _) = as_binary(right);
                assert_eq!(cmp_op, Less);
            }

            #[test]
            fn recovers_inside_parentheses() {
                let expr = parse_error_recovery("(== 1)");
                let inner = as_grouping(&expr);
                let (_, right) = assert_missing_left_binary(inner, EqualEqual);
                assert_eq!(as_number(right), 1);
            }
        }

        mod comma {
            use super::*;

            #[test]
            fn recovers_bare_comma_with_number() {
                let expr = parse_error_recovery(", 1");
                let (_, right) = assert_missing_left_binary(&expr, Comma);
                assert_eq!(as_number(right), 1);
            }

            #[test]
            fn rhs_parses_comma_expression() {
                let expr = parse_error_recovery(", 1, 2");
                let (_, right) = assert_missing_left_binary(&expr, Comma);
                let (left, op, right) = as_binary(right);
                assert_eq!(op, Comma);
                assert_eq!(as_number(left), 1);
                assert_eq!(as_number(right), 2);
            }

            #[test]
            fn rhs_parses_ternary_within_comma_expression() {
                let expr = parse_error_recovery(", true ? 1 : 2");
                let (_, right) = assert_missing_left_binary(&expr, Comma);
                let (condition, then_branch, else_branch) = as_ternary(right);
                assert!(matches!(condition, Expr::SyntaxLiteral(Literal::True)));
                assert_eq!(as_number(then_branch), 1);
                assert_eq!(as_number(else_branch), 2);
            }
        }

        mod recovery {
            use super::*;

            #[test]
            fn returns_ok_for_recovered_expression() {
                let result = parse_with_error("+ 1");
                assert!(result.is_ok());
            }

            #[test]
            fn still_returns_syntax_tree() {
                let result = parse_with_error("+ 1");
                assert!(result.is_ok());
            }

            #[test]
            fn valid_expressions_remain_unaffected() {
                parse_ok("1 + 2 * 3");
            }

            #[test]
            fn bare_operator_without_rhs_still_fails() {
                parse_fails("+");
            }

            #[test]
            fn middle_operand_still_requires_expression() {
                parse_fails("1 * + 2");
            }
        }
    }
}
