use crate::rlox::eval::RuntimeError::{IncorrectOperator, IncorrectType};
use crate::rlox::parser::{
    Expr,
    Literal::{False, FloatNumber, Nil, Number, StringL, True},
};
use crate::rlox::token::{Token, TokenType};
use std::fmt::{Display, Formatter};
use std::ops::Neg;
use thiserror::Error;

#[derive(Debug, PartialEq)]
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
}

fn is_truthy(value: &Value) -> bool {
    !matches!(value, Value::Boolean(false) | Value::Nil)
}

fn is_equal(value1: &Value, value2: &Value) -> bool {
    value1 == value2
}

pub fn eval_expr(exp: Expr) -> Result<Value, RuntimeError> {
    match exp {
        Expr::SyntaxLiteral(literal) => Ok(match literal {
            Number(value) => Value::Number(value as f64),
            FloatNumber(value) => Value::Number(value),
            True => Value::Boolean(true),
            False => Value::Boolean(false),
            Nil => Value::Nil,
            StringL(value) => Value::Str(value),
        }),
        Expr::Binary { left, op, right } => {
            let left_value = eval_expr(*left)?;
            let right_value = eval_expr(*right)?;
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
                    TokenType::Plus => Ok(Value::Number(left + right)),
                    TokenType::Minus => Ok(Value::Number(left - right)),
                    TokenType::Star => Ok(Value::Number(left * right)),
                    TokenType::Slash => Ok(Value::Number(left / right)),
                    TokenType::Greater => Ok(Value::Boolean(left > right)),
                    TokenType::GreaterEqual => Ok(Value::Boolean(left >= right)),
                    TokenType::Less => Ok(Value::Boolean(left < right)),
                    TokenType::LessEqual => Ok(Value::Boolean(left <= right)),
                    _other => Err(IncorrectOperator(op)),
                }
            } else if let Value::Str(ref left) = left_value
                && let Value::Str(right) = right_value
            {
                match &op.token_type {
                    TokenType::Plus => Ok(Value::Str(format!("{left}{right}"))),
                    _other => Err(IncorrectOperator(op)),
                }
            } else {
                Err(IncorrectType(left_value, op))
            }
        }
        Expr::Ternary { .. } => {
            todo!()
        }
        Expr::Unary { op, right } => {
            let right_value = eval_expr(*right)?;
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
        Expr::Grouping { expr } => eval_expr(*expr),
        Expr::Operator { .. } => {
            todo!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rlox::parser::tests::parse_ok;
    use crate::rlox::parser::{
        self,
        Expr::{Binary, Operator, SyntaxLiteral, Ternary, Unary},
        Literal,
    };
    use crate::rlox::token::TokenType::{
        And, Bang, Colon, Comma, Dot, Equal, Greater, GreaterEqual, If, LeftParen, Less, Minus, Or,
        Plus, Question, Semicolon, Slash, Star, While,
    };
    use crate::rlox::token::{Token, TokenType};
    use std::ops::Deref;

    fn eval_ok(input: &str) -> Value {
        eval_expr(parse_ok(input)).unwrap_or_else(|e| panic!("eval failed for {input:?}: {e}"))
    }

    fn eval_parse(input: &str) -> Result<Value, RuntimeError> {
        eval_expr(parse_ok(input))
    }

    fn eval_expr_ok(expr: Expr) -> Value {
        eval_expr(expr).unwrap_or_else(|e| panic!("eval failed: {e}"))
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
        assert_incorrect_type(eval_expr(expr), expected_type);
    }

    fn assert_incorrect_operator(result: Result<Value, RuntimeError>, expected: TokenType) {
        match result {
            Err(IncorrectOperator(op)) => assert_eq!(op.token_type, expected),
            Err(other) => panic!("expected IncorrectOperand, got {other:?}"),
            Ok(value) => panic!("expected operator error, got value {value:?}"),
        }
    }

    fn assert_incorrect_operator_expr(expr: Expr, expected: TokenType) {
        assert_incorrect_operator(eval_expr(expr), expected);
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
        fn division_by_zero_produces_infinity() {
            let result = eval_ok("1 / 0");
            match result {
                Value::Number(n) => assert!(n.is_infinite() && n.is_sign_positive()),
                other => panic!("expected positive infinity, got {other:?}"),
            }
        }

        #[test]
        fn zero_divided_by_zero_produces_nan() {
            let result = eval_ok("0 / 0");
            match result {
                Value::Number(n) => assert!(n.is_nan()),
                other => panic!("expected NaN, got {other:?}"),
            }
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
        #[should_panic]
        fn ternary_expression() {
            let expr = Ternary {
                predicate: Box::new(SyntaxLiteral(Literal::True)),
                left: Box::new(lit_number(1)),
                right: Box::new(lit_number(2)),
            };
            let _ = eval_expr(expr);
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
            #[should_panic(expected = "not yet implemented")]
            fn bare_plus_operator() {
                let expr = Operator { token: token(Plus) };
                let _ = eval_expr(expr);
            }

            #[test]
            #[should_panic(expected = "not yet implemented")]
            fn bare_minus_operator() {
                let expr = Operator {
                    token: token(Minus),
                };
                let _ = eval_expr(expr);
            }

            #[test]
            #[should_panic(expected = "not yet implemented")]
            fn bare_bang_operator() {
                let expr = Operator { token: token(Bang) };
                let _ = eval_expr(expr);
            }

            #[test]
            #[should_panic(expected = "not yet implemented")]
            fn bare_star_operator() {
                let expr = Operator { token: token(Star) };
                let _ = eval_expr(expr);
            }
        }

        mod error_messages {
            use super::*;

            #[test]
            fn binary_operator_error_message() {
                let expr = binary(lit_number(1), Bang, lit_number(2));
                let err = eval_expr(expr).unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Incorrect operator `Token { token_type: Bang, lexeme: \"\", line: 1 }`!"
                );
            }

            #[test]
            fn unary_operator_error_message() {
                let expr = unary(Plus, lit_number(1));
                let err = eval_expr(expr).unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Incorrect operator `Token { token_type: Plus, lexeme: \"\", line: 1 }`!"
                );
            }

            #[test]
            #[should_panic(expected = "not yet implemented")]
            fn operator_placeholder_error_message() {
                let expr = Operator { token: token(Star) };
                let _err = eval_expr(expr).unwrap_err();
            }
        }
    }

    mod type_errors {
        use super::*;

        mod addition {
            use super::*;

            #[test]
            fn number_plus_string() {
                assert_incorrect_type_for("1 + \"two\"", "number");
            }

            #[test]
            fn string_plus_number() {
                assert_incorrect_type_for("\"one\" + 2", "string");
            }

            #[test]
            fn number_plus_boolean() {
                assert_incorrect_type_for("1 + true", "number");
            }

            #[test]
            fn boolean_plus_number() {
                assert_incorrect_type_for("false + 1", "boolean");
            }

            #[test]
            fn number_plus_nil() {
                assert_incorrect_type_for("1 + nil", "number");
            }

            #[test]
            fn nil_plus_number() {
                assert_incorrect_type_for("nil + 1", "nil");
            }

            #[test]
            fn string_plus_boolean() {
                assert_incorrect_type_for("\"a\" + true", "string");
            }

            #[test]
            fn boolean_plus_string() {
                assert_incorrect_type_for("true + \"a\"", "boolean");
            }

            #[test]
            fn string_plus_nil() {
                assert_incorrect_type_for("\"a\" + nil", "string");
            }

            #[test]
            fn nil_plus_string() {
                assert_incorrect_type_for("nil + \"a\"", "nil");
            }

            #[test]
            fn boolean_plus_boolean() {
                assert_incorrect_type_for("true + false", "boolean");
            }

            #[test]
            fn nil_plus_nil() {
                assert_incorrect_type_for("nil + nil", "nil");
            }

            #[test]
            fn boolean_plus_nil() {
                assert_incorrect_type_for("true + nil", "boolean");
            }

            #[test]
            fn number_plus_string_via_manual_construction() {
                let expr = binary(lit_number(1), Plus, lit_string("x"));
                assert_incorrect_type_expr(expr, "number");
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
                assert_incorrect_type_for("1 - \"b\"", "number");
            }

            #[test]
            fn string_minus_number() {
                assert_incorrect_type_for("\"a\" - 1", "string");
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
                assert_incorrect_type_for("2 * \"a\"", "number");
            }

            #[test]
            fn string_times_number() {
                assert_incorrect_type_for("\"a\" * 2", "string");
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
                assert_incorrect_type_for("8 / \"b\"", "number");
            }

            #[test]
            fn string_divided_by_number() {
                assert_incorrect_type_for("\"a\" / 2", "string");
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
                assert_incorrect_type_for("1 < \"2\"", "number");
            }

            #[test]
            fn string_less_than_number() {
                assert_incorrect_type_for("\"1\" < 2", "string");
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
                assert_incorrect_type_for("\"a\" > 1", "string");
            }

            #[test]
            fn number_greater_equal_string() {
                assert_incorrect_type_for("1 >= \"1\"", "number");
            }

            #[test]
            fn comparison_between_number_and_string_via_manual_construction() {
                let expr = binary(lit_number(1), Less, lit_string("2"));
                assert_incorrect_type_expr(expr, "number");
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
                let err = eval_parse("1 + \"two\"").unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Unexpected operand Number(1.0) for operator Plus \"+\"!"
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
                    "Unexpected operand Boolean(true) for operator Less \"<\"!"
                );
            }

            #[test]
            fn unary_negation_error_includes_expected_type() {
                let err = eval_parse("-nil").unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Unexpected operand Nil for operator Minus \"-\"!"
                );
            }

            #[test]
            fn mixed_addition_reports_right_operand_type() {
                let err = eval_parse("1 + true").unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Unexpected operand Number(1.0) for operator Plus \"+\"!"
                );
            }

            #[test]
            fn invalid_left_operand_reported_first_for_subtraction() {
                let err = eval_parse("true - 1").unwrap_err();
                assert_eq!(
                    err.to_string(),
                    "Unexpected operand Boolean(true) for operator Minus \"-\"!"
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
}
