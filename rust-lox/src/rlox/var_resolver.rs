use crate::rlox::parser::{Block, Statement};
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

    fn resolve_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Print(_) => {}
            Statement::ExprStatement(_) => {}
            Statement::DeclarationStatement(name, init) => {
                // self.declare()
                todo!()
            }
            Statement::BlockStatement(_) => {}
            Statement::IfStatement { .. } => {}
            Statement::WhileStatement { .. } => {}
            Statement::BreakStatement(_) => {}
            Statement::ReturnStatement(_, _) => {}
            Statement::FunctionStatement { .. } => {}
        }
    }
    fn resolve_block(&mut self, block: &Block) {
        self.begin_scope();
        for statement in &block.statements {
            self.resolve_statement(statement)
        }
        self.end_scope();
    }

    fn declare(&mut self, name: &Token) {
        if let Some(map) = self.scopes.last_mut() {
            map.insert(name.lexeme.clone(), false);
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

    // fn define(&mut self, token: &Token) {
    //     self.scopes.last().map()
    // }
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
