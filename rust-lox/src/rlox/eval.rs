use crate::rlox::parser::Literal::{False, FloatNumber, Nil, Number, StringL, True};
use crate::rlox::parser::{Expr, Literal};
use crate::rlox::token::TokenType;
use std::ops::Neg;

#[derive(PartialEq)]
pub enum Value {
    Number(f64),
    Str(String),
    Boolean(bool),
    Nil,
}

fn is_truthy(value: &Value) -> bool {
    !matches!(value, Value::Boolean(false) | Value::Nil)
}

fn is_equal(value1: &Value, value2: &Value) -> bool {
    value1 == value2
}

pub fn eval_expr(exp: Expr) -> Value {
    match exp {
        Expr::SyntaxLiteral(literal) => match literal {
            Number(value) => Value::Number(value as f64),
            FloatNumber(value) => Value::Number(value),
            True => Value::Boolean(true),
            False => Value::Boolean(false),
            Nil => Value::Nil,
            StringL(value) => Value::Str(value),
        },
        Expr::Binary { left, op, right } => {
            let left_value = eval_expr(*left);
            let right_value = eval_expr(*right);
            if op.token_type == TokenType::Equal {
                return Value::Boolean(is_equal(&left_value, &right_value));
            }
            // TODO laziness for boolean OR AND
            if let Value::Number(left) = left_value
                && let Value::Number(right) = right_value
            {
                match op.token_type {
                    TokenType::Plus => Value::Number(left + right),
                    TokenType::Minus => Value::Number(left - right),
                    TokenType::Star => Value::Number(left * right),
                    TokenType::Slash => Value::Number(left / right),
                    TokenType::Greater => Value::Boolean(left > right),
                    TokenType::GreaterEqual => Value::Boolean(left >= right),
                    TokenType::Less => Value::Boolean(left < right),
                    TokenType::LessEqual => Value::Boolean(left <= right),
                    _ => todo!("runtime error"),
                }
            } else if let Value::Str(left) = left_value
                && let Value::Str(right) = right_value
            {
                match op.token_type {
                    TokenType::Plus => Value::Str(format!("{left}{right}")),
                    _ => todo!("runtime error"),
                }
            } else {
                todo!("runtime error")
            }
        }
        Expr::Ternary { .. } => {
            todo!()
        }
        Expr::Unary { op, right } => {
            let right_value = eval_expr(*right);
            match op.token_type {
                TokenType::Bang => Value::Boolean(is_truthy(&right_value)),
                TokenType::Minus => {
                    if let Value::Number(value) = right_value {
                        return Value::Number(value.neg());
                    }
                    todo!("Implement error when negation is applied to non-numerical type")
                }
                _ => todo!("Implement error when unary op type if not ! or -"),
            }
        }
        Expr::Grouping { expr } => eval_expr(*expr),
        Expr::Operator { .. } => {
            todo!()
        }
    }
}
