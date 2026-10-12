use crate::rlox::interpreter::RloxError;
use crate::rlox::parser::{Block, Expr, IdentifierExpr, Statement};
use crate::rlox::token::Token;
use std::collections::HashMap;

pub struct Resolver {
    scopes: Vec<HashMap<String, bool>>,
}

impl Resolver {
    pub fn new() -> Resolver {
        Resolver { scopes: vec![] }
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new())
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn resolve_statement(&mut self, statement: &mut Statement) -> Result<(), RloxError> {
        match statement {
            Statement::Print(expr) => {
                self.resolve_expression(expr)?;
            }
            Statement::ExprStatement(expr) => {
                self.resolve_expression(expr)?;
            }
            Statement::DeclarationStatement(name, init) => {
                self.declare(name);
                if let Some(expr) = init {
                    self.resolve_expression(expr)?
                }
                self.define(name);
            }
            Statement::BlockStatement(block) => {
                self.resolve_block(block)?;
            }
            Statement::IfStatement {
                expr,
                then,
                else_stmnt,
            } => {
                self.resolve_expression(expr)?;
                self.resolve_statement(then)?;
                if let Some(else_st) = else_stmnt {
                    self.resolve_statement(else_st)?
                }
            }
            Statement::WhileStatement { expr, body } => {
                self.resolve_expression(expr)?;
                self.resolve_statement(body)?;
            }
            Statement::BreakStatement(_) => {}
            Statement::ReturnStatement(_, expr) => {
                if let Some(expr) = expr {
                    self.resolve_expression(expr)?;
                }
            }
            Statement::FunctionStatement { name, args, body } => {
                self.declare_and_define(&name.token);

                self.begin_scope();
                for arg in args {
                    self.declare_and_define(arg);
                }
                self.resolve_block(body)?;
                self.end_scope();
            }
        }
        Ok(())
    }

    fn resolve_identifier(&mut self, ident: &mut IdentifierExpr) -> Result<(), RloxError> {
        let token_lexeme = &ident.token.lexeme;
        if let Some(value) = self.get_value(token_lexeme)
            && !value
        {
            Err(RloxError::ResolverError(ident.token.clone()))
        } else {
            let token_lexeme = &ident.token.lexeme;
            for (i, scope) in self.scopes.iter().rev().enumerate() {
                if scope.get(token_lexeme).is_some() {
                    ident.hops = Some(i as u16);
                    break;
                }
            }
            Ok(())
        }
    }

    pub fn resolve_expression(&mut self, expr: &mut Expr) -> Result<(), RloxError> {
        match expr {
            Expr::SyntaxLiteral(_) => {}
            Expr::IdentifierExpression(ident_expr) => {
                self.resolve_identifier(ident_expr)?;
            }
            Expr::Binary { left, op, right } => {
                self.resolve_expression(left)?;
                self.resolve_expression(right)?;
            }
            Expr::Ternary {
                predicate,
                left,
                right,
            } => {
                self.resolve_expression(predicate)?;
                self.resolve_expression(left)?;
                self.resolve_expression(right)?;
            }
            Expr::Unary { op: _op, right } => {
                self.resolve_expression(right)?;
            }
            Expr::Grouping { expr } => {
                self.resolve_expression(expr)?;
            }
            Expr::AssignmentExpression { left, right } => {
                self.resolve_identifier(left)?;
                self.resolve_expression(right)?;
            }
            Expr::MissingLeftOperand { .. } => {}
            Expr::CallableExpression {
                paren: _paren,
                args,
                callee,
            } => {
                for arg in args {
                    self.resolve_expression(arg)?;
                }
                self.resolve_expression(callee)?;
            }
            Expr::FunctionExpression { args: _args, body } => {
                self.resolve_block(body)?;
            }
        }
        Ok(())
    }
    fn resolve_block(&mut self, block: &mut Block) -> Result<(), RloxError> {
        self.begin_scope();
        for statement in &mut block.statements {
            self.resolve_statement(statement)?
        }
        self.end_scope();
        Ok(())
    }

    fn declare(&mut self, name: &Token) {
        if let Some(map) = self.scopes.last_mut() {
            map.insert(name.lexeme.clone(), false);
        }
    }

    fn declare_and_define(&mut self, name: &Token) {
        if let Some(map) = self.scopes.last_mut() {
            map.insert(name.lexeme.clone(), true);
        }
    }

    fn define(&mut self, token: &Token) {
        if let Some(map) = self.scopes.last_mut()
            && let Some(val) = map.get_mut(&token.lexeme)
        {
            *val = true
        }
    }

    fn get_value(&self, value: &str) -> Option<&bool> {
        self.scopes.last().and_then(|s| s.get(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rlox::token::TokenType::StringToken;

    #[test]
    fn test() {
        let mut res = Resolver::new();

        res.begin_scope();
        let name = String::from("test");
        let token = Token::new(StringToken(name.clone()), name.clone(), 0);
        res.declare(&token);

        let value = res.get_value(&name);
        assert!(value.is_some());
        assert!(!value.unwrap());

        res.define(&token);
        let value = res.get_value(&name);
        assert!(value.is_some());
        assert!(value.unwrap());
    }
}
