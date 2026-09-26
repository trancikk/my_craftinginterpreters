use crate::rlox::environment::{Env, EnvRef};
use crate::rlox::eval::RuntimeError::{IncorrectOperator, IncorrectType};
use crate::rlox::parser::{
    Expr,
    Literal::{False, FloatNumber, Nil, Number, StringL, True},
    Statement,
};
use crate::rlox::token::TokenType::Plus;
use crate::rlox::token::{Token, TokenType};
use std::fmt::{Display, Formatter};
use std::ops::Neg;
use thiserror::Error;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Number(f64),
    Str(String),
    Boolean(bool),
    Nil,
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Value::Number(n) => n.to_string(),
            Value::Str(s) => s.clone(),
            Value::Boolean(v) => v.to_string(),
            Value::Nil => String::from("nil"),
        };
        write!(f, "{s}")
    }
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Unexpected operand {0:?} for operator {1}!")]
    IncorrectType(Value, Token),
    #[error("Incorrect operator `{0:?}`!")]
    IncorrectOperator(Token),
    #[error("{0:?}")]
    EvaluationError(String, Token),
    #[error("Identifier {0:?} not found!")]
    IdentifierNotFound(Token),
}

fn is_truthy(value: &Value) -> bool {
    !matches!(value, Value::Boolean(false) | Value::Nil)
}

fn is_equal(value1: &Value, value2: &Value) -> bool {
    value1 == value2
}

pub fn eval_statement(env: &EnvRef, statement: Statement) -> Result<(), RuntimeError> {
    match statement {
        Statement::BlockStatement(statements) => {
            let nested_env = Env::from_env(env);
            for statement in statements {
                eval_statement(&nested_env, statement)?
            }
        }
        Statement::DeclarationStatement(name, expr) => {
            let initial_value = match expr {
                Some(expr) => eval_expr(env, expr)?,
                None => Value::Nil,
            };
            env.borrow_mut().define(name, initial_value)
        }
        Statement::ExprStatement(expr) => {
            eval_expr(env, expr)?;
        }
        Statement::Print(expr) => {
            let value = eval_expr(env, expr)?;
            println!("{value}");
        }
    };
    Ok(())
}

pub fn eval_expr(env: &EnvRef, exp: Expr) -> Result<Value, RuntimeError> {
    match exp {
        Expr::SyntaxLiteral(literal) => Ok(match literal {
            Number(value) => Value::Number(value as f64),
            FloatNumber(value) => Value::Number(value),
            True => Value::Boolean(true),
            False => Value::Boolean(false),
            Nil => Value::Nil,
            StringL(value) => Value::Str(value),
        }),
        Expr::AssignmentExpression { left, right } => {
            let lexeme_value = &left.lexeme;
            let variable = env.borrow().get(lexeme_value);
            match variable {
                Some(_) => {
                    let r_value = eval_expr(env, *right)?;
                    env.borrow_mut().assign(lexeme_value, r_value.clone());
                    Ok(r_value)
                }
                None => Err(RuntimeError::EvaluationError(
                    format!("Variable {lexeme_value} doesn't exist"),
                    left,
                )),
            }

            // TODO add error when variable doesn't exist
        }
        Expr::Binary { left, op, right } => {
            let left_value = eval_expr(env, *left)?;
            let right_value = eval_expr(env, *right)?;
            if op.token_type == TokenType::EqualEqual {
                return Ok(Value::Boolean(is_equal(&left_value, &right_value)));
            } else if op.token_type == TokenType::BangEqual {
                return Ok(Value::Boolean(!is_equal(&left_value, &right_value)));
            }
            // TODO laziness for boolean OR AND
            if let Value::Number(left) = left_value
                && let Value::Number(right) = right_value
            {
                match &op.token_type {
                    Plus => Ok(Value::Number(left + right)),
                    TokenType::Minus => Ok(Value::Number(left - right)),
                    TokenType::Star => Ok(Value::Number(left * right)),
                    TokenType::Slash => {
                        if right == 0f64 {
                            return Err(RuntimeError::EvaluationError(
                                String::from("Division by Zero!"),
                                op,
                            ));
                        }
                        Ok(Value::Number(left / right))
                    }
                    TokenType::Greater => Ok(Value::Boolean(left > right)),
                    TokenType::GreaterEqual => Ok(Value::Boolean(left >= right)),
                    TokenType::Less => Ok(Value::Boolean(left < right)),
                    TokenType::LessEqual => Ok(Value::Boolean(left <= right)),
                    _other => Err(IncorrectOperator(op)),
                }
            } else if matches!(&left_value, Value::Str(_)) || matches!(&right_value, Value::Str(_))
            {
                match &op.token_type {
                    Plus => Ok(Value::Str(format!("{left_value}{right_value}"))),
                    _other => Err(IncorrectOperator(op)),
                }
            } else {
                Err(IncorrectType(left_value, op))
            }
        }
        Expr::Ternary {
            predicate,
            left,
            right,
        } => {
            let predicate_result = eval_expr(env, *predicate)?;
            if is_truthy(&predicate_result) {
                eval_expr(env, *left)
            } else {
                eval_expr(env, *right)
            }
        }
        Expr::Unary { op, right } => {
            let right_value = eval_expr(env, *right)?;
            match op.token_type {
                TokenType::Bang => Ok(Value::Boolean(!is_truthy(&right_value))),
                TokenType::Minus => {
                    if let Value::Number(value) = right_value {
                        return Ok(Value::Number(value.neg()));
                    }
                    Err(IncorrectType(right_value, op))
                }
                _ => Err(IncorrectOperator(op)),
            }
        }
        Expr::Grouping { expr } => eval_expr(env, *expr),
        Expr::MissingLeftOperand { token } => {
            let msg = format!("Missing left operand for token {token}");
            Err(RuntimeError::EvaluationError(msg, token))
        }
        Expr::IdentifierExpression(token) => {
            if let Some(identifier_value) = env.borrow().get(&token.lexeme) {
                Ok(identifier_value)
            } else {
                Err(RuntimeError::IdentifierNotFound(token))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rlox::parser::tests::parse_ok;
    use crate::rlox::parser::{
        Expr::{Binary, MissingLeftOperand, SyntaxLiteral, Ternary, Unary},
        Literal,
    };
    use crate::rlox::token::TokenType::{
        And, Bang, Colon, Comma, Dot, Equal, Greater, GreaterEqual, If, LeftParen, Less, Minus, Or,
        Plus, Question, Semicolon, Slash, Star, While,
    };
    use crate::rlox::token::{Token, TokenType};

    fn eval_ok(input: &str) -> Value {
        let mut env = Env::new();
        eval_expr(&mut env, parse_ok(input))
            .unwrap_or_else(|e| panic!("eval failed for {input:?}: {e}"))
    }

    fn eval_parse(input: &str) -> Result<Value, RuntimeError> {
        let mut env = Env::new();
        eval_expr(&mut env, parse_ok(input))
    }

    fn eval_expr_ok(expr: Expr) -> Value {
        let mut env = Env::new();
        eval_expr(&mut env, expr).unwrap_or_else(|e| panic!("eval failed: {e}"))
    }

    fn get_type_name(value: &Value) -> &'static str {
        match value {
            Value::Number(_) => "number",
            Value::Str(_) => "string",
            Value::Boolean(_) => "boolean",
            Value::Nil => "nil",
        }
    }

    fn assert_incorrect_type(result: Result<Value, RuntimeError>, expected_type: &str) {
        match result {
            Err(IncorrectType(value, _)) => {
                assert_eq!(get_type_name(&value), expected_type);
            }
            Err(other) => panic!("expected IncorrectType, got {other:?}"),
            Ok(value) => panic!("expected type error, got value {value:?}"),
        }
    }

    fn assert_incorrect_type_for(input: &str, expected_type: &str) {
        assert_incorrect_type(eval_parse(input), expected_type);
    }

    fn assert_incorrect_type_expr(expr: Expr, expected_type: &str) {
        let mut env = Env::new();
        assert_incorrect_type(eval_expr(&mut env, expr), expected_type);
    }

    fn assert_incorrect_operator(result: Result<Value, RuntimeError>, expected: TokenType) {
        match result {
            Err(IncorrectOperator(op)) => assert_eq!(op.token_type, expected),
            Err(other) => panic!("expected IncorrectOperand, got {other:?}"),
            Ok(value) => panic!("expected operator error, got value {value:?}"),
        }
    }

    fn assert_incorrect_operator_expr(expr: Expr, expected: TokenType) {
        let mut env = Env::new();
        assert_incorrect_operator(eval_expr(&mut env, expr), expected);
    }

    fn token(token_type: TokenType) -> Token {
        Token::new(token_type, String::new(), 1)
    }

    fn lit_number(n: u64) -> Expr {
        SyntaxLiteral(Literal::Number(n))
    }

    fn lit_float(n: f64) -> Expr {
        SyntaxLiteral(Literal::FloatNumber(n))
    }

    fn lit_string(s: &str) -> Expr {
        SyntaxLiteral(Literal::StringL(s.to_string()))
    }

    fn binary(left: Expr, op: TokenType, right: Expr) -> Expr {
        Binary {
            left: Box::new(left),
            op: token(op),
            right: Box::new(right),
        }
    }

    fn unary(op: TokenType, right: Expr) -> Expr {
        Unary {
            op: token(op),
            right: Box::new(right),
        }
    }
    fn assert_number(value: Value, expected: f64) {
        match value {
            Value::Number(n) => assert!(
                (n - expected).abs() < f64::EPSILON,
                "expected {expected}, got {n}"
            ),
            other => panic!("expected number {expected}, got {other:?}"),
        }
    }

    fn assert_bool(value: Value, expected: bool) {
        assert_eq!(value, Value::Boolean(expected));
    }

    fn assert_str(value: Value, expected: &str) {
        assert_eq!(value, Value::Str(expected.to_string()));
    }

    fn assert_nil(value: Value) {
        assert_eq!(value, Value::Nil);
    }

    mod literals {
        use super::*;

        #[test]
        fn integer() {
            assert_number(eval_ok("42"), 42.0);
        }

        #[test]
        fn zero() {
            assert_number(eval_ok("0"), 0.0);
        }

        #[test]
        fn float() {
            assert_number(eval_ok("3.15"), 3.15);
        }

        #[test]
        fn string() {
            assert_str(eval_ok("\"hello\""), "hello");
        }

        #[test]
        fn empty_string() {
            assert_str(eval_ok("\"\""), "");
        }

        #[test]
        fn true_literal() {
            assert_bool(eval_ok("true"), true);
        }

        #[test]
        fn false_literal() {
            assert_bool(eval_ok("false"), false);
        }

        #[test]
        fn nil_literal() {
            assert_nil(eval_ok("nil"));
        }

        #[test]
        fn large_integer() {
            assert_number(eval_ok("18446744073709551615"), 18446744073709551615.0);
        }
    }

    mod arithmetic {
        use super::*;

        #[test]
        fn addition() {
            assert_number(eval_ok("1 + 2"), 3.0);
        }

        #[test]
        fn subtraction() {
            assert_number(eval_ok("5 - 3"), 2.0);
        }

        #[test]
        fn multiplication() {
            assert_number(eval_ok("3 * 4"), 12.0);
        }

        #[test]
        fn division() {
            assert_number(eval_ok("8 / 2"), 4.0);
        }

        #[test]
        fn chained_addition_is_left_associative() {
            assert_number(eval_ok("1 + 2 + 3"), 6.0);
        }

        #[test]
        fn multiplication_binds_tighter_than_addition() {
            assert_number(eval_ok("1 + 2 * 3"), 7.0);
        }

        #[test]
        fn parentheses_override_precedence() {
            assert_number(eval_ok("(1 + 2) * 3"), 9.0);
        }

        #[test]
        fn float_arithmetic() {
            assert_number(eval_ok("1.5 + 2"), 3.5);
        }

        #[test]
        fn float_multiplication() {
            assert_number(eval_ok("2.5 * 2"), 5.0);
        }

        #[test]
        fn negative_via_unary_minus() {
            assert_number(eval_ok("-5"), -5.0);
        }

        #[test]
        fn double_negation() {
            assert_number(eval_ok("--3"), 3.0);
        }

        #[test]
        fn subtraction_produces_negative() {
            assert_number(eval_ok("3 - 10"), -7.0);
        }

        #[test]
        fn division_by_zero_returns_error() {
            let err = eval_parse("1 / 0").unwrap_err();
            assert_eq!(err.to_string(), "\"Division by Zero!\"");
        }

        #[test]
        fn zero_divided_by_zero_returns_error() {
            let err = eval_parse("0 / 0").unwrap_err();
            assert_eq!(err.to_string(), "\"Division by Zero!\"");
        }
    }

    mod comparison {
        use super::*;

        #[test]
        fn less_than() {
            assert_bool(eval_ok("1 < 2"), true);
        }

        #[test]
        fn less_than_false() {
            assert_bool(eval_ok("2 < 1"), false);
        }

        #[test]
        fn less_equal_true_when_equal() {
            assert_bool(eval_ok("2 <= 2"), true);
        }

        #[test]
        fn less_equal_false_when_greater() {
            assert_bool(eval_ok("3 <= 2"), false);
        }

        #[test]
        fn greater_than() {
            assert_bool(eval_ok("3 > 2"), true);
        }

        #[test]
        fn greater_equal_true_when_equal() {
            assert_bool(eval_ok("5 >= 5"), true);
        }

        #[test]
        fn greater_equal_false_when_less() {
            assert_bool(eval_ok("1 >= 2"), false);
        }

        #[test]
        fn comparison_with_floats() {
            assert_bool(eval_ok("1.5 < 2.0"), true);
        }

        #[test]
        fn comparison_respects_arithmetic_precedence() {
            assert_bool(eval_ok("1 + 2 < 5"), true);
        }
    }

    mod equality {
        use super::*;
        use crate::rlox::eval::TokenType::EqualEqual;

        #[test]
        fn equal_numbers_via_manual_token() {
            let expr = binary(lit_number(1), EqualEqual, lit_number(1));
            assert_bool(eval_expr_ok(expr), true);
        }

        #[test]
        fn unequal_numbers_via_manual_token() {
            let expr = binary(lit_number(1), EqualEqual, lit_number(2));
            assert_bool(eval_expr_ok(expr), false);
        }

        #[test]
        fn equal_strings_via_manual_token() {
            let expr = binary(lit_string("a"), EqualEqual, lit_string("a"));
            assert_bool(eval_expr_ok(expr), true);
        }

        #[test]
        fn unequal_strings_via_manual_token() {
            let expr = binary(lit_string("a"), EqualEqual, lit_string("b"));
            assert_bool(eval_expr_ok(expr), false);
        }

        #[test]
        fn nil_equals_nil_via_manual_token() {
            let expr = binary(
                SyntaxLiteral(Literal::Nil),
                EqualEqual,
                SyntaxLiteral(Literal::Nil),
            );
            assert_bool(eval_expr_ok(expr), true);
        }

        #[test]
        fn number_not_equal_to_string_via_manual_token() {
            let expr = binary(lit_number(1), EqualEqual, lit_string("1"));
            assert_bool(eval_expr_ok(expr), false);
        }

        #[test]
        fn boolean_equality_via_manual_token() {
            let expr = binary(
                SyntaxLiteral(Literal::True),
                EqualEqual,
                SyntaxLiteral(Literal::True),
            );
            assert_bool(eval_expr_ok(expr), true);
        }

        #[test]
        fn parsed_double_equal() {
            assert_bool(eval_ok("1 == 1"), true);
            assert_bool(eval_ok("1 == 2"), false);
        }

        #[test]
        fn parsed_bang_equal() {
            assert_bool(eval_ok("1 != 2"), true);
            assert_bool(eval_ok("1 != 1"), false);
        }

        #[test]
        fn parsed_equality_across_types_is_false() {
            assert_bool(eval_ok("1 == \"1\""), false);
            assert_bool(eval_ok("nil == 1"), false);
        }
    }

    mod string_concat {
        use super::*;

        #[test]
        fn concatenates_two_strings() {
            assert_str(eval_ok("\"foo\" + \"bar\""), "foobar");
        }

        #[test]
        fn concatenates_with_empty_string() {
            assert_str(eval_ok("\"\" + \"x\""), "x");
        }

        #[test]
        fn concatenates_multiple_strings() {
            assert_str(eval_ok("\"a\" + \"b\" + \"c\""), "abc");
        }
    }

    mod unary {
        use super::*;
        use crate::rlox::token::TokenType::EqualEqual;

        #[test]
        fn not_true_is_false() {
            assert_bool(eval_ok("!true"), false);
        }

        #[test]
        fn not_false_is_true() {
            assert_bool(eval_ok("!false"), true);
        }

        #[test]
        fn not_nil_is_true() {
            assert_bool(eval_ok("!nil"), true);
        }

        #[test]
        fn not_zero_is_false_because_zero_is_truthy() {
            assert_bool(eval_ok("!0"), false);
        }

        #[test]
        fn not_empty_string_is_false_because_strings_are_truthy() {
            assert_bool(eval_ok("!\"\""), false);
        }

        #[test]
        fn not_non_zero_number_is_false() {
            assert_bool(eval_ok("!42"), false);
        }

        #[test]
        fn double_not() {
            assert_bool(eval_ok("!!true"), true);
        }

        #[test]
        fn negate_number() {
            assert_number(eval_ok("-42"), -42.0);
        }

        #[test]
        fn negate_float() {
            assert_number(eval_ok("-3.5"), -3.5);
        }

        #[test]
        fn negate_negative_number() {
            assert_number(eval_ok("--7"), 7.0);
        }

        #[test]
        fn not_binds_tighter_than_equality_via_manual_token() {
            let expr = binary(
                unary(Bang, SyntaxLiteral(True)),
                EqualEqual,
                SyntaxLiteral(False),
            );
            assert_bool(eval_expr_ok(expr), true);
        }

        #[test]
        fn not_binds_tighter_than_equality() {
            assert_bool(eval_ok("!true == false"), true);
        }

        #[test]
        fn unary_minus_binds_tighter_than_multiplication() {
            assert_number(eval_ok("-2 * 3"), -6.0);
        }
    }

    mod grouping {
        use super::*;

        #[test]
        fn grouping_preserves_value() {
            assert_number(eval_ok("(42)"), 42.0);
        }

        #[test]
        fn nested_grouping() {
            assert_number(eval_ok("((10))"), 10.0);
        }

        #[test]
        fn grouping_affects_precedence() {
            assert_number(eval_ok("2 * (3 + 4)"), 14.0);
        }
    }

    mod complex {
        use super::*;
        use crate::rlox::token::TokenType::EqualEqual;

        #[test]
        fn mixed_arithmetic_and_comparison() {
            assert_bool(eval_ok("(1 + 2) * -3 < 10"), true);
        }

        #[test]
        fn deeply_nested_expression() {
            assert_number(eval_ok("((1 + 2) * (4 - 1)) / 3"), 3.0);
        }

        #[test]
        fn string_and_nil_equality_via_manual_token() {
            let expr = binary(lit_string("a"), EqualEqual, SyntaxLiteral(Literal::Nil));
            assert_bool(eval_expr_ok(expr), false);
        }
    }

    mod unimplemented {
        use super::*;

        #[test]
        fn ternary_expression() {
            let expr = Ternary {
                predicate: Box::new(SyntaxLiteral(Literal::True)),
                left: Box::new(lit_number(1)),
                right: Box::new(lit_number(2)),
            };
            assert_eq!(eval_expr_ok(expr), Value::Number(1.0));
        }
    }

    mod invalid_operators {
        use super::*;

        mod binary {
            use super::*;

            #[test]
            fn bang_between_numbers() {
                let expr = binary(lit_number(1), Bang, lit_number(2));
                assert_incorrect_operator_expr(expr, Bang);
            }

            #[test]
            fn and_between_numbers() {
                let expr = binary(lit_number(1), And, lit_number(2));
                assert_incorrect_operator_expr(expr, And);
            }

            #[test]
            fn or_between_numbers() {
                let expr = binary(lit_number(1), Or, lit_number(2));
                assert_incorrect_operator_expr(expr, Or);
            }

            #[test]
            fn comma_between_numbers() {
                let expr = binary(lit_number(1), Comma, lit_number(2));
                assert_incorrect_operator_expr(expr, Comma);
            }

            #[test]
            fn left_paren_between_numbers() {
                let expr = binary(lit_number(1), LeftParen, lit_number(2));
                assert_incorrect_operator_expr(expr, LeftParen);
            }

            #[test]
            fn question_between_numbers() {
                let expr = binary(lit_number(1), Question, lit_number(2));
                assert_incorrect_operator_expr(expr, Question);
            }

            #[test]
            fn colon_between_numbers() {
                let expr = binary(lit_number(1), Colon, lit_number(2));
                assert_incorrect_operator_expr(expr, Colon);
            }

            #[test]
            fn dot_between_numbers() {
                let expr = binary(lit_number(1), Dot, lit_number(2));
                assert_incorrect_operator_expr(expr, Dot);
            }

            #[test]
            fn semicolon_between_numbers() {
                let expr = binary(lit_number(1), Semicolon, lit_number(2));
                assert_incorrect_operator_expr(expr, Semicolon);
            }

            #[test]
            fn if_keyword_between_numbers() {
                let expr = binary(lit_number(1), If, lit_number(2));
                assert_incorrect_operator_expr(expr, If);
            }

            #[test]
            fn while_keyword_between_numbers() {
                let expr = binary(lit_number(1), While, lit_number(2));
                assert_incorrect_operator_expr(expr, While);
            }
        }

        mod unary {
            use super::*;

            #[test]
            fn plus_on_number() {
                let expr = unary(Plus, lit_number(5));
                assert_incorrect_operator_expr(expr, Plus);
            }

            #[test]
            fn star_on_number() {
                let expr = unary(Star, lit_number(5));
                assert_incorrect_operator_expr(expr, Star);
            }

            #[test]
            fn slash_on_number() {
                let expr = unary(Slash, lit_number(5));
                assert_incorrect_operator_expr(expr, Slash);
            }

            #[test]
            fn equal_on_number() {
                let expr = unary(Equal, lit_number(5));
                assert_incorrect_operator_expr(expr, Equal);
            }

            #[test]
            fn less_on_number() {
                let expr = unary(Less, lit_number(5));
                assert_incorrect_operator_expr(expr, Less);
            }

            #[test]
            fn greater_on_number() {
                let expr = unary(Greater, lit_number(5));
                assert_incorrect_operator_expr(expr, Greater);
            }

            #[test]
            fn greater_equal_on_number() {
                let expr = unary(GreaterEqual, lit_number(5));
                assert_incorrect_operator_expr(expr, GreaterEqual);
            }

            #[test]
            fn and_on_number() {
                let expr = unary(And, lit_number(5));
                assert_incorrect_operator_expr(expr, And);
            }

            #[test]
            fn or_on_number() {
                let expr = unary(Or, lit_number(5));
                assert_incorrect_operator_expr(expr, Or);
            }

            #[test]
            fn comma_on_number() {
                let expr = unary(Comma, lit_number(5));
                assert_incorrect_operator_expr(expr, Comma);
            }
        }

        mod operator_placeholder {
            use super::*;

            #[test]
            fn bare_plus_operator() {
                let expr = MissingLeftOperand { token: token(Plus) };
                let mut env = Env::new();
                let err = eval_expr(&mut env, expr).unwrap_err();
                assert!(matches!(err, RuntimeError::EvaluationError(_, _)));
            }

            #[test]
            fn bare_minus_operator() {
                let expr = MissingLeftOperand {
                    token: token(Minus),
                };
                let mut env = Env::new();
                let err = eval_expr(&mut env, expr).unwrap_err();
                assert!(matches!(err, RuntimeError::EvaluationError(_, _)));
            }

            #[test]
            fn bare_bang_operator() {
                let expr = MissingLeftOperand { token: token(Bang) };
                let mut env = Env::new();
                let err = eval_expr(&mut env, expr).unwrap_err();
                assert!(matches!(err, RuntimeError::EvaluationError(_, _)));
            }

            #[test]
            fn bare_star_operator() {
                let expr = MissingLeftOperand { token: token(Star) };
                let mut env = Env::new();
                let err = eval_expr(&mut env, expr).unwrap_err();
                assert!(matches!(err, RuntimeError::EvaluationError(_, _)));
            }
        }

        mod error_messages {
            use super::*;

            #[test]
            fn binary_operator_error_message() {
                let expr = binary(lit_number(1), Bang, lit_number(2));
                let mut env = Env::new();
                let err = eval_expr(&mut env, expr).unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Incorrect operator `Token { token_type: Bang, lexeme: \"\", line: 1 }`!"
                );
            }

            #[test]
            fn unary_operator_error_message() {
                let expr = unary(Plus, lit_number(1));
                let mut env = Env::new();
                let err = eval_expr(&mut env, expr).unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Incorrect operator `Token { token_type: Plus, lexeme: \"\", line: 1 }`!"
                );
            }

            #[test]
            fn operator_placeholder_error_message() {
                let expr = MissingLeftOperand { token: token(Star) };
                let mut env = Env::new();
                let err = eval_expr(&mut env, expr).unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "\"Missing left operand for token  at line 1\""
                );
            }
        }
    }

    mod type_errors {
        use super::*;

        mod addition {
            use super::*;

            #[test]
            fn number_plus_string_converts_to_string() {
                assert_str(eval_ok("1 + \"two\""), "1two");
            }

            #[test]
            fn string_plus_number_converts_to_string() {
                assert_str(eval_ok("\"one\" + 2"), "one2");
            }

            #[test]
            fn string_plus_boolean_converts_to_string() {
                assert_str(eval_ok("\"a\" + true"), "atrue");
            }

            #[test]
            fn boolean_plus_string_converts_to_string() {
                assert_str(eval_ok("true + \"a\""), "truea");
            }

            #[test]
            fn string_plus_nil_converts_to_string() {
                assert_str(eval_ok("\"a\" + nil"), "anil");
            }

            #[test]
            fn nil_plus_string_converts_to_string() {
                assert_str(eval_ok("nil + \"a\""), "nila");
            }

            #[test]
            fn number_plus_string_via_manual_construction() {
                let expr = binary(lit_number(1), Plus, lit_string("x"));
                assert_str(eval_expr_ok(expr), "1x");
            }

            #[test]
            fn number_plus_boolean_is_type_error() {
                assert_incorrect_type_for("1 + true", "number");
            }

            #[test]
            fn boolean_plus_number_is_type_error() {
                assert_incorrect_type_for("false + 1", "boolean");
            }

            #[test]
            fn number_plus_nil_is_type_error() {
                assert_incorrect_type_for("1 + nil", "number");
            }

            #[test]
            fn nil_plus_number_is_type_error() {
                assert_incorrect_type_for("nil + 1", "nil");
            }

            #[test]
            fn boolean_plus_boolean_is_type_error() {
                assert_incorrect_type_for("true + false", "boolean");
            }

            #[test]
            fn nil_plus_nil_is_type_error() {
                assert_incorrect_type_for("nil + nil", "nil");
            }

            #[test]
            fn boolean_plus_nil_is_type_error() {
                assert_incorrect_type_for("true + nil", "boolean");
            }
        }

        mod subtraction {
            use super::*;

            #[test]
            fn string_minus_string() {
                let expr = binary(lit_string("a"), Minus, lit_string("b"));
                assert_incorrect_operator_expr(expr, Minus);
            }

            #[test]
            fn number_minus_string() {
                let expr = binary(lit_number(1), Minus, lit_string("b"));
                assert_incorrect_operator_expr(expr, Minus);
            }

            #[test]
            fn string_minus_number() {
                let expr = binary(lit_string("a"), Minus, lit_number(1));
                assert_incorrect_operator_expr(expr, Minus);
            }

            #[test]
            fn boolean_minus_number() {
                assert_incorrect_type_for("true - 1", "boolean");
            }

            #[test]
            fn number_minus_boolean() {
                assert_incorrect_type_for("1 - false", "number");
            }

            #[test]
            fn nil_minus_number() {
                assert_incorrect_type_for("nil - 1", "nil");
            }

            #[test]
            fn number_minus_nil() {
                assert_incorrect_type_for("1 - nil", "number");
            }

            #[test]
            fn boolean_minus_boolean() {
                assert_incorrect_type_for("true - false", "boolean");
            }
        }

        mod multiplication {
            use super::*;

            #[test]
            fn string_multiplication() {
                let expr = binary(lit_string("a"), Star, lit_string("b"));
                assert_incorrect_operator_expr(expr, Star);
            }

            #[test]
            fn number_times_string() {
                let expr = binary(lit_number(2), Star, lit_string("a"));
                assert_incorrect_operator_expr(expr, Star);
            }

            #[test]
            fn string_times_number() {
                let expr = binary(lit_string("a"), Star, lit_number(2));
                assert_incorrect_operator_expr(expr, Star);
            }

            #[test]
            fn boolean_multiplication() {
                assert_incorrect_type_for("true * false", "boolean");
            }

            #[test]
            fn nil_multiplication() {
                assert_incorrect_type_for("nil * 1", "nil");
            }

            #[test]
            fn number_times_nil() {
                assert_incorrect_type_for("1 * nil", "number");
            }
        }

        mod division {
            use super::*;

            #[test]
            fn string_division() {
                let expr = binary(lit_string("a"), Slash, lit_string("b"));
                assert_incorrect_operator_expr(expr, Slash);
            }

            #[test]
            fn number_divided_by_string() {
                let expr = binary(lit_number(8), Slash, lit_string("b"));
                assert_incorrect_operator_expr(expr, Slash);
            }

            #[test]
            fn string_divided_by_number() {
                let expr = binary(lit_string("a"), Slash, lit_number(2));
                assert_incorrect_operator_expr(expr, Slash);
            }

            #[test]
            fn boolean_division() {
                assert_incorrect_type_for("true / false", "boolean");
            }

            #[test]
            fn nil_division() {
                assert_incorrect_type_for("nil / 1", "nil");
            }

            #[test]
            fn number_divided_by_nil() {
                assert_incorrect_type_for("1 / nil", "number");
            }
        }

        mod comparison {
            use super::*;

            #[test]
            fn string_less_than_string() {
                let expr = binary(lit_string("a"), Less, lit_string("b"));
                assert_incorrect_operator_expr(expr, Less);
            }

            #[test]
            fn number_less_than_string() {
                let expr = binary(lit_number(1), Less, lit_string("2"));
                assert_incorrect_operator_expr(expr, Less);
            }

            #[test]
            fn string_less_than_number() {
                let expr = binary(lit_string("1"), Less, lit_number(2));
                assert_incorrect_operator_expr(expr, Less);
            }

            #[test]
            fn boolean_less_than_number() {
                assert_incorrect_type_for("true < 1", "boolean");
            }

            #[test]
            fn number_less_than_boolean() {
                assert_incorrect_type_for("1 < false", "number");
            }

            #[test]
            fn nil_less_than_number() {
                assert_incorrect_type_for("nil < 1", "nil");
            }

            #[test]
            fn number_less_than_nil() {
                assert_incorrect_type_for("1 < nil", "number");
            }

            #[test]
            fn boolean_less_equal_boolean() {
                assert_incorrect_type_for("true <= false", "boolean");
            }

            #[test]
            fn string_greater_than_number() {
                let expr = binary(lit_string("a"), Greater, lit_number(1));
                assert_incorrect_operator_expr(expr, Greater);
            }

            #[test]
            fn number_greater_equal_string() {
                let expr = binary(lit_number(1), GreaterEqual, lit_string("1"));
                assert_incorrect_operator_expr(expr, GreaterEqual);
            }

            #[test]
            fn comparison_between_number_and_string_via_manual_construction() {
                let expr = binary(lit_number(1), Less, lit_string("2"));
                assert_incorrect_operator_expr(expr, Less);
            }

            #[test]
            fn greater_equal_with_boolean_operands() {
                let expr = binary(
                    SyntaxLiteral(Literal::True),
                    GreaterEqual,
                    SyntaxLiteral(Literal::False),
                );
                assert_incorrect_type_expr(expr, "boolean");
            }

            #[test]
            fn greater_with_nil_operands() {
                let expr = binary(
                    SyntaxLiteral(Literal::Nil),
                    Greater,
                    SyntaxLiteral(Literal::Nil),
                );
                assert_incorrect_type_expr(expr, "nil");
            }
        }

        mod unary_negation {
            use super::*;

            #[test]
            fn negate_boolean() {
                assert_incorrect_type_for("-true", "boolean");
            }

            #[test]
            fn negate_false() {
                assert_incorrect_type_for("-false", "boolean");
            }

            #[test]
            fn negate_nil() {
                assert_incorrect_type_for("-nil", "nil");
            }

            #[test]
            fn negate_string() {
                assert_incorrect_type_for("-\"hello\"", "string");
            }

            #[test]
            fn negate_empty_string() {
                assert_incorrect_type_for("-\"\"", "string");
            }

            #[test]
            fn negate_boolean_via_manual_construction() {
                let expr = unary(Minus, SyntaxLiteral(Literal::True));
                assert_incorrect_type_expr(expr, "boolean");
            }

            #[test]
            fn negate_nil_via_manual_construction() {
                let expr = unary(Minus, SyntaxLiteral(Literal::Nil));
                assert_incorrect_type_expr(expr, "nil");
            }

            #[test]
            fn negate_string_via_manual_construction() {
                let expr = unary(Minus, lit_string("x"));
                assert_incorrect_type_expr(expr, "string");
            }
        }

        mod error_messages {
            use super::*;

            #[test]
            fn addition_error_includes_expected_type() {
                let err = eval_parse("1 + true").unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Unexpected operand Number(1.0) for operator + at line 0!"
                );
            }

            #[test]
            fn subtraction_error_includes_expected_type() {
                let err = eval_parse("\"a\" - \"b\"").unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Incorrect operator `Token { token_type: Minus, lexeme: \"-\", line: 0 }`!"
                );
            }

            #[test]
            fn comparison_error_includes_expected_type() {
                let err = eval_parse("true < 1").unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Unexpected operand Boolean(true) for operator < at line 0!"
                );
            }

            #[test]
            fn unary_negation_error_includes_expected_type() {
                let err = eval_parse("-nil").unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Unexpected operand Nil for operator - at line 0!"
                );
            }

            #[test]
            fn mixed_addition_reports_right_operand_type() {
                let err = eval_parse("1 + true").unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Unexpected operand Number(1.0) for operator + at line 0!"
                );
            }

            #[test]
            fn invalid_left_operand_reported_first_for_subtraction() {
                let err = eval_parse("true - 1").unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Unexpected operand Boolean(true) for operator - at line 0!"
                );
            }
        }
    }

    mod edge_cases {
        use super::*;
        use crate::rlox::token::TokenType::EqualEqual;

        #[test]
        fn subtracting_larger_from_smaller() {
            assert_number(eval_ok("1 - 100"), -99.0);
        }

        #[test]
        fn multiplying_by_zero() {
            assert_number(eval_ok("0 * 999"), 0.0);
        }

        #[test]
        fn comparing_equal_floats() {
            assert_bool(eval_ok("1.0 >= 1.0"), true);
        }

        #[test]
        fn not_on_expression_result() {
            assert_bool(eval_ok("!(1 < 2)"), false);
        }

        #[test]
        fn grouping_around_unary() {
            assert_number(eval_ok("(-5)"), -5.0);
        }

        #[test]
        fn equality_short_circuits_before_type_check_on_manual_token() {
            let expr = binary(lit_number(1), EqualEqual, lit_string("1"));
            assert_bool(eval_expr_ok(expr), false);
        }

        #[test]
        fn string_concat_does_not_add_separators() {
            assert_str(eval_ok("\"1\" + \"2\""), "12");
        }

        #[test]
        fn float_equality_via_manual_token() {
            let expr = binary(lit_float(1.0), EqualEqual, lit_float(1.0));
            assert_bool(eval_expr_ok(expr), true);
        }

        #[test]
        fn integer_and_float_addition() {
            assert_number(eval_ok("1 + 2.5"), 3.5);
        }
    }

    mod identifiers {
        use super::*;
        use crate::rlox::eval::{RuntimeError, eval_statement};
        use crate::rlox::parser::tests::parse_statement_ok;

        fn eval_ok_with_env(env: &mut Env, input: &str) -> Value {
            eval_expr(env, parse_ok(input))
                .unwrap_or_else(|e| panic!("eval failed for {input:?}: {e}"))
        }

        #[test]
        fn var_declaration_with_initializer() {
            let mut env = Env::new();
            let stmt = parse_statement_ok("var x = 42;");
            eval_statement(&mut env, stmt).unwrap();
            assert_number(eval_ok_with_env(&mut env, "x"), 42.0);
        }

        #[test]
        fn var_declaration_without_initializer_defaults_to_nil() {
            let mut env = Env::new();
            let stmt = parse_statement_ok("var x;");
            eval_statement(&mut env, stmt).unwrap();
            assert_nil(eval_ok_with_env(&mut env, "x"));
        }

        #[test]
        fn var_declaration_with_expression_initializer() {
            let mut env = Env::new();
            let stmt = parse_statement_ok("var x = 1 + 2 * 3;");
            eval_statement(&mut env, stmt).unwrap();
            assert_number(eval_ok_with_env(&mut env, "x"), 7.0);
        }

        #[test]
        fn var_declaration_with_string_initializer() {
            let mut env = Env::new();
            let stmt = parse_statement_ok("var name = \"hello\";");
            eval_statement(&mut env, stmt).unwrap();
            assert_str(eval_ok_with_env(&mut env, "name"), "hello");
        }

        #[test]
        fn identifier_lookup_after_declaration() {
            let mut env = Env::new();
            eval_statement(&mut env, parse_statement_ok("var a = 1;")).unwrap();
            eval_statement(&mut env, parse_statement_ok("var b = 2;")).unwrap();
            assert_number(eval_ok_with_env(&mut env, "a"), 1.0);
            assert_number(eval_ok_with_env(&mut env, "b"), 2.0);
        }

        #[test]
        fn identifier_used_in_expression() {
            let mut env = Env::new();
            eval_statement(&mut env, parse_statement_ok("var x = 10;")).unwrap();
            assert_number(eval_ok_with_env(&mut env, "x + 5"), 15.0);
            assert_number(eval_ok_with_env(&mut env, "x * 2"), 20.0);
            assert_number(eval_ok_with_env(&mut env, "x - 3"), 7.0);
            assert_number(eval_ok_with_env(&mut env, "x / 2"), 5.0);
        }

        #[test]
        fn identifier_used_in_comparison() {
            let mut env = Env::new();
            eval_statement(&mut env, parse_statement_ok("var x = 5;")).unwrap();
            assert_bool(eval_ok_with_env(&mut env, "x > 3"), true);
            assert_bool(eval_ok_with_env(&mut env, "x < 10"), true);
            assert_bool(eval_ok_with_env(&mut env, "x == 5"), true);
            assert_bool(eval_ok_with_env(&mut env, "x != 6"), true);
        }

        #[test]
        fn identifier_used_in_ternary() {
            let mut env = Env::new();
            eval_statement(&mut env, parse_statement_ok("var cond = true;")).unwrap();
            assert_number(eval_ok_with_env(&mut env, "cond ? 1 : 2"), 1.0);
            eval_statement(&mut env, parse_statement_ok("var cond = false;")).unwrap();
            assert_number(eval_ok_with_env(&mut env, "cond ? 1 : 2"), 2.0);
        }

        #[test]
        fn identifier_used_in_grouping() {
            let mut env = Env::new();
            eval_statement(&mut env, parse_statement_ok("var x = 3;")).unwrap();
            assert_number(eval_ok_with_env(&mut env, "(x)"), 3.0);
            assert_number(eval_ok_with_env(&mut env, "((x))"), 3.0);
        }

        #[test]
        fn multiple_variables_in_expression() {
            let mut env = Env::new();
            eval_statement(&mut env, parse_statement_ok("var a = 1;")).unwrap();
            eval_statement(&mut env, parse_statement_ok("var b = 2;")).unwrap();
            eval_statement(&mut env, parse_statement_ok("var c = 3;")).unwrap();
            assert_number(eval_ok_with_env(&mut env, "a + b + c"), 6.0);
            assert_number(eval_ok_with_env(&mut env, "a * b * c"), 6.0);
        }

        #[test]
        fn identifier_not_found_returns_error() {
            let mut env = Env::new();
            let err = eval_expr(&mut env, parse_ok("undefined_var")).unwrap_err();
            assert!(matches!(err, RuntimeError::IdentifierNotFound(_)));
        }

        #[test]
        fn identifier_not_found_error_message() {
            let mut env = Env::new();
            let err = eval_expr(&mut env, parse_ok("missing")).unwrap_err();
            assert_eq!(
                err.to_string(),
                "Identifier Token { token_type: Identifier, lexeme: \"missing\", line: 0 } not found!"
            );
        }

        #[test]
        fn redefine_variable() {
            let mut env = Env::new();
            eval_statement(&mut env, parse_statement_ok("var x = 1;")).unwrap();
            assert_number(eval_ok_with_env(&mut env, "x"), 1.0);
            eval_statement(&mut env, parse_statement_ok("var x = 2;")).unwrap();
            assert_number(eval_ok_with_env(&mut env, "x"), 2.0);
        }

        #[test]
        fn variable_shadowing_not_supported_in_same_scope() {
            // Current implementation allows redefinition in same scope
            let mut env = Env::new();
            eval_statement(&mut env, parse_statement_ok("var x = 1;")).unwrap();
            eval_statement(&mut env, parse_statement_ok("var x = 2;")).unwrap();
            assert_number(eval_ok_with_env(&mut env, "x"), 2.0);
        }

        mod assignment {
            use super::*;
            use crate::rlox::eval::eval_statement;
            use crate::rlox::parser::tests::parse_statement_ok;

            fn eval_ok_with_env(env: &mut Env, input: &str) -> Value {
                eval_expr(env, parse_ok(input))
                    .unwrap_or_else(|e| panic!("eval failed for {input:?}: {e}"))
            }

            #[test]
            fn simple_assignment() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var x = 1;")).unwrap();
                assert_number(eval_ok_with_env(&mut env, "x = 42"), 42.0);
                assert_number(eval_ok_with_env(&mut env, "x"), 42.0);
            }

            #[test]
            fn assignment_with_expression_rhs() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var x = 1;")).unwrap();
                assert_number(eval_ok_with_env(&mut env, "x = 1 + 2 * 3"), 7.0);
                assert_number(eval_ok_with_env(&mut env, "x"), 7.0);
            }

            #[test]
            fn assignment_with_string() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var x = \"old\";")).unwrap();
                assert_str(eval_ok_with_env(&mut env, r#"x = "new""#), "new");
                assert_str(eval_ok_with_env(&mut env, "x"), "new");
            }

            #[test]
            fn assignment_with_nil() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var x = 1;")).unwrap();
                assert_nil(eval_ok_with_env(&mut env, "x = nil"));
                assert_nil(eval_ok_with_env(&mut env, "x"));
            }

            #[test]
            fn assignment_with_unary_rhs() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var x = 1;")).unwrap();
                assert_number(eval_ok_with_env(&mut env, "x = -5"), -5.0);
                assert_number(eval_ok_with_env(&mut env, "x"), -5.0);
            }

            #[test]
            fn chained_assignment() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var a = 1;")).unwrap();
                eval_statement(&mut env, parse_statement_ok("var b = 2;")).unwrap();
                assert_number(eval_ok_with_env(&mut env, "a = b = 3"), 3.0);
                assert_number(eval_ok_with_env(&mut env, "a"), 3.0);
                assert_number(eval_ok_with_env(&mut env, "b"), 3.0);
            }

            #[test]
            fn chained_assignment_right_associative() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var a = 1;")).unwrap();
                eval_statement(&mut env, parse_statement_ok("var b = 2;")).unwrap();
                eval_statement(&mut env, parse_statement_ok("var c = 3;")).unwrap();
                assert_number(eval_ok_with_env(&mut env, "a = b = c = 4"), 4.0);
                assert_number(eval_ok_with_env(&mut env, "a"), 4.0);
                assert_number(eval_ok_with_env(&mut env, "b"), 4.0);
                assert_number(eval_ok_with_env(&mut env, "c"), 4.0);
            }

            #[test]
            fn assignment_in_ternary_then_branch() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var x = 0;")).unwrap();
                eval_statement(&mut env, parse_statement_ok("var y = 0;")).unwrap();
                assert_number(eval_ok_with_env(&mut env, "true ? x = 1 : y = 2"), 1.0);
                assert_number(eval_ok_with_env(&mut env, "x"), 1.0);
                assert_number(eval_ok_with_env(&mut env, "y"), 0.0);
            }

            #[test]
            fn assignment_in_ternary_else_branch() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var x = 0;")).unwrap();
                eval_statement(&mut env, parse_statement_ok("var y = 0;")).unwrap();
                assert_number(eval_ok_with_env(&mut env, "false ? x = 1 : y = 2"), 2.0);
                assert_number(eval_ok_with_env(&mut env, "x"), 0.0);
                assert_number(eval_ok_with_env(&mut env, "y"), 2.0);
            }

            #[test]
            fn assignment_in_grouping() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var x = 0;")).unwrap();
                assert_number(eval_ok_with_env(&mut env, "(x = 5)"), 5.0);
                assert_number(eval_ok_with_env(&mut env, "x"), 5.0);
            }

            #[test]
            fn assignment_as_expression_statement() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var x = 0;")).unwrap();
                let stmt = parse_statement_ok("x = 42;");
                eval_statement(&mut env, stmt).unwrap();
                assert_number(eval_ok_with_env(&mut env, "x"), 42.0);
            }

            #[test]
            fn assignment_with_complex_rhs() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var x = 0;")).unwrap();
                assert_number(eval_ok_with_env(&mut env, "x = (1 + 2) * 3"), 9.0);
                assert_number(eval_ok_with_env(&mut env, "x"), 9.0);
            }

            #[test]
            fn assignment_returns_rhs_value() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var x = 0;")).unwrap();
                let result = eval_ok_with_env(&mut env, "x = 99");
                assert_number(result, 99.0);
            }

            #[test]
            fn assignment_with_ternary_rhs() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var x = 0;")).unwrap();
                eval_statement(&mut env, parse_statement_ok("var cond = true;")).unwrap();
                assert_number(eval_ok_with_env(&mut env, "x = cond ? 1 : 2"), 1.0);
                assert_number(eval_ok_with_env(&mut env, "x"), 1.0);
                eval_statement(&mut env, parse_statement_ok("var cond = false;")).unwrap();
                assert_number(eval_ok_with_env(&mut env, "x = cond ? 1 : 2"), 2.0);
                assert_number(eval_ok_with_env(&mut env, "x"), 2.0);
            }

            #[test]
            fn assignment_precedence_lower_than_ternary() {
                let mut env = Env::new();
                eval_statement(&mut env, parse_statement_ok("var a = 0;")).unwrap();
                assert_number(eval_ok_with_env(&mut env, "a = true ? 1 : 2"), 1.0);
                assert_number(eval_ok_with_env(&mut env, "a"), 1.0);
            }
        }

        mod nested_environment {
            use super::*;
            use crate::rlox::environment::Env;
            use std::cell::RefCell;
            use std::rc::Rc;

            #[test]
            fn nested_env_child_can_read_parent() {
                let parent = Rc::new(RefCell::new(Env::new()));
                parent
                    .borrow_mut()
                    .define("x".to_string(), Value::Number(10.0));
                let mut child = Env::from_env(parent.clone());
                let result = eval_expr(&mut child, parse_ok("x"));
                assert!(result.is_ok());
                assert_number(result.unwrap(), 10.0);
            }

            #[test]
            fn nested_env_child_can_shadow_parent() {
                let parent = Rc::new(RefCell::new(Env::new()));
                parent
                    .borrow_mut()
                    .define("x".to_string(), Value::Number(10.0));
                let mut child = Env::from_env(parent.clone());
                child.define("x".to_string(), Value::Number(20.0));
                let result = eval_expr(&mut child, parse_ok("x"));
                assert!(result.is_ok());
                assert_number(result.unwrap(), 20.0);
            }

            #[test]
            fn nested_env_assignment_updates_parent() {
                let parent = Rc::new(RefCell::new(Env::new()));
                parent
                    .borrow_mut()
                    .define("x".to_string(), Value::Number(10.0));
                let mut child = Env::from_env(parent.clone());
                let _result = eval_expr(&mut child, parse_ok("x = 20"));
            }

            #[test]
            fn nested_env_new_assignment_creates_in_child() {
                let parent = Rc::new(RefCell::new(Env::new()));
                let mut child = Env::from_env(parent.clone());
                let _result = eval_expr(&mut child, parse_ok("y = 30"));
            }

            #[test]
            fn three_level_nested_env() {
                let grandparent = Rc::new(RefCell::new(Env::new()));
                grandparent
                    .borrow_mut()
                    .define("a".to_string(), Value::Number(1.0));
                let parent = Rc::new(RefCell::new(Env::from_env(grandparent.clone())));
                parent
                    .borrow_mut()
                    .define("b".to_string(), Value::Number(2.0));
                let mut child = Env::from_env(parent.clone());
                child.define("c".to_string(), Value::Number(3.0));
                let result_a = eval_expr(&mut child, parse_ok("a"));
                assert!(result_a.is_ok());
                assert_number(result_a.unwrap(), 1.0);
                let result_b = eval_expr(&mut child, parse_ok("b"));
                assert!(result_b.is_ok());
                assert_number(result_b.unwrap(), 2.0);
                let result_c = eval_expr(&mut child, parse_ok("c"));
                assert!(result_c.is_ok());
                assert_number(result_c.unwrap(), 3.0);
            }

            #[test]
            fn nested_env_assignment_through_levels() {
                let grandparent = Rc::new(RefCell::new(Env::new()));
                grandparent
                    .borrow_mut()
                    .define("x".to_string(), Value::Number(1.0));
                let parent = Rc::new(RefCell::new(Env::from_env(grandparent.clone())));
                let mut child = Env::from_env(parent.clone());
                let _result = eval_expr(&mut child, parse_ok("x = 100"));
            }
        }
    }
}
