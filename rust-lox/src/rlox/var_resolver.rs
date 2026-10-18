use crate::rlox::interpreter::RloxError;
use crate::rlox::parser::{Block, Expr, IdentifierExpr, Statement};
use crate::rlox::token::Token;
use crate::rlox::var_resolver::Resolution::{Global, Local};
use std::collections::HashMap;

pub struct Resolver {
    scopes: Vec<HashMap<String, bool>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Resolution {
    Unresolved,
    Global,
    Local(u16),
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
                self.declare(name)?;
                if let Some(expr) = init {
                    self.resolve_expression(expr)?
                }
                self.define(name);
            }
            Statement::BlockStatement(block) => {
                self.begin_scope();
                self.resolve_block(block)?;
                self.end_scope();
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
                self.declare_and_define(&name.token)?;

                self.begin_scope();
                for arg in args {
                    self.declare_and_define(arg)?;
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
            let mut resolution = Global;
            for (i, scope) in self.scopes.iter().rev().enumerate() {
                if scope.get(token_lexeme).is_some() {
                    resolution = Local(i as u16);
                    break;
                }
            }
            ident.hops = resolution;
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
            Expr::FunctionExpression { args, body } => {
                self.begin_scope();
                for arg in args {
                    self.declare_and_define(arg)?;
                }
                self.resolve_block(body)?;
                self.end_scope();
            }
        }
        Ok(())
    }
    fn resolve_block(&mut self, block: &mut Block) -> Result<(), RloxError> {
        // self.begin_scope();
        for statement in &mut block.statements {
            self.resolve_statement(statement)?
        }
        // self.end_scope();
        Ok(())
    }

    fn declare(&mut self, name: &Token) -> Result<(), RloxError> {
        if let Some(map) = self.scopes.last_mut()
            && !map.contains_key(&name.lexeme)
        {
            map.insert(name.lexeme.clone(), false);
            return Ok(());
        }
        Err(RloxError::ResolverError(name.clone()))
    }

    fn exist(&mut self, name: &Token) -> bool {
        if let Some(map) = self.scopes.last_mut() {
            map.contains_key(&name.lexeme)
        } else {
            false
        }
    }

    fn declare_and_define(&mut self, name: &Token) -> Result<(), RloxError> {
        if let Some(map) = self.scopes.last_mut()
            && !map.contains_key(&name.lexeme)
        {
            map.insert(name.lexeme.clone(), true);
            return Ok(());
        }
        Err(RloxError::ResolverError(name.clone()))
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
    use crate::rlox::environment::Env;
    use crate::rlox::eval::{Value, eval_expr};
    use crate::rlox::interpreter::RloxError;
    use crate::rlox::parser::{ExecFlow, Parser};
    use crate::rlox::scanner::Scanner;
    use crate::rlox::var_resolver::Resolution::{Global, Local, Unresolved};

    fn parse_expression(source: &str) -> Expr {
        Parser::new(Scanner::new(source.lines()))
            .parse_expression()
            .unwrap_or_else(|error| panic!("failed to parse expression {source:?}: {error}"))
    }

    fn parse_statement(source: &str) -> Statement {
        Parser::new(Scanner::new(source.lines()))
            .parse_statements()
            .unwrap_or_else(|error| panic!("failed to parse statement {source:?}: {error}"))
    }

    fn resolve_expression(source: &str) -> Result<Expr, RloxError> {
        let mut expression = parse_expression(source);
        Resolver::new().resolve_expression(&mut expression)?;
        Ok(expression)
    }

    fn resolve_statement(source: &str) -> Result<Statement, RloxError> {
        let mut statement = parse_statement(source);
        Resolver::new().resolve_statement(&mut statement)?;
        Ok(statement)
    }

    fn expression_references(expression: &Expr, references: &mut Vec<(String, Resolution)>) {
        match expression {
            Expr::SyntaxLiteral(_) | Expr::MissingLeftOperand { .. } => {}
            Expr::IdentifierExpression(identifier) => {
                references.push((identifier.token.lexeme.clone(), identifier.hops.clone()))
            }
            Expr::Binary { left, right, .. } => {
                expression_references(left, references);
                expression_references(right, references);
            }
            Expr::Ternary {
                predicate,
                left,
                right,
            } => {
                expression_references(predicate, references);
                expression_references(left, references);
                expression_references(right, references);
            }
            Expr::Unary { right, .. } => expression_references(right, references),
            Expr::Grouping { expr } => expression_references(expr, references),
            Expr::AssignmentExpression { left, right } => {
                references.push((left.token.lexeme.clone(), left.hops.clone()));
                expression_references(right, references);
            }
            Expr::CallableExpression { callee, args, .. } => {
                expression_references(callee, references);
                for argument in args {
                    expression_references(argument, references);
                }
            }
            Expr::FunctionExpression { body, .. } => block_references(body, references),
        }
    }

    fn statement_references(statement: &Statement, references: &mut Vec<(String, Resolution)>) {
        match statement {
            Statement::Print(expression) | Statement::ExprStatement(expression) => {
                expression_references(expression, references);
            }
            Statement::DeclarationStatement(_, initializer) => {
                if let Some(initializer) = initializer {
                    expression_references(initializer, references);
                }
            }
            Statement::BlockStatement(block) => block_references(block, references),
            Statement::IfStatement {
                expr,
                then,
                else_stmnt,
            } => {
                expression_references(expr, references);
                statement_references(then, references);
                if let Some(else_statement) = else_stmnt {
                    statement_references(else_statement, references);
                }
            }
            Statement::WhileStatement { expr, body } => {
                expression_references(expr, references);
                statement_references(body, references);
            }
            Statement::BreakStatement(_) => {}
            Statement::ReturnStatement(_, expression) => {
                if let Some(expression) = expression {
                    expression_references(expression, references);
                }
            }
            Statement::FunctionStatement { body, .. } => block_references(body, references),
        }
    }

    fn block_references(block: &Block, references: &mut Vec<(String, Resolution)>) {
        for statement in &block.statements {
            statement_references(statement, references);
        }
    }

    fn references(statement: &Statement) -> Vec<(String, Resolution)> {
        let mut references = Vec::new();
        statement_references(statement, &mut references);
        references
    }

    fn expression_resolution(expression: &Expr) -> &Resolution {
        let Expr::IdentifierExpression(identifier) = expression else {
            panic!("expected identifier expression, got {expression:?}");
        };
        &identifier.hops
    }

    #[test]
    fn parser_leaves_identifier_unresolved_until_resolver_runs() {
        let expression = parse_expression("name");

        assert_eq!(expression_resolution(&expression), &Unresolved);
    }

    #[test]
    fn unresolved_lookup_uses_legacy_nearest_environment_fallback() {
        let global = Env::new_global();
        let local = Env::from_env(&global);
        Env::define(&global, "name".to_string(), Value::Number(1.0));
        Env::define(&local, "name".to_string(), Value::Number(2.0));

        let expression = parse_expression("name");
        assert_eq!(expression_resolution(&expression), &Unresolved);
        assert_eq!(eval_expr(&local, expression).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn unresolved_assignment_uses_legacy_nearest_environment_fallback() {
        let global = Env::new_global();
        let local = Env::from_env(&global);
        Env::define(&global, "name".to_string(), Value::Number(1.0));
        Env::define(&local, "name".to_string(), Value::Number(2.0));

        let expression = parse_expression("name = 3");
        assert_eq!(eval_expr(&local, expression).unwrap(), Value::Number(3.0));
        assert_eq!(
            Env::get_at(&local, "name", &Local(0)),
            Some(Value::Number(3.0))
        );
        assert_eq!(
            Env::get_at(&local, "name", &Global),
            Some(Value::Number(1.0))
        );
    }

    #[test]
    fn unresolved_closure_keeps_legacy_dynamic_lookup_behavior() {
        let global = Env::new_global();
        Env::define(
            &global,
            "value".to_string(),
            Value::Str("global".to_string()),
        );
        Env::define(&global, "observed".to_string(), Value::Nil);
        let statement = parse_statement(
            r#"{
                fun read() { observed = value; }
                read();
                var value = "block";
                read();
            }"#,
        );

        crate::rlox::eval::eval_statement(&global, statement).unwrap();

        assert_eq!(
            Env::get(&global, "observed"),
            Some(Value::Str("block".to_string()))
        );
    }

    #[test]
    fn resolver_marks_top_level_reference_global() {
        let expression = resolve_expression("name").unwrap();

        assert_eq!(expression_resolution(&expression), &Global);
    }

    #[test]
    fn global_resolution_bypasses_a_nearer_runtime_binding() {
        let global = Env::new_global();
        let local = Env::from_env(&global);
        Env::define(&global, "name".to_string(), Value::Number(1.0));
        Env::define(&local, "name".to_string(), Value::Number(2.0));

        let expression = resolve_expression("name").unwrap();
        assert_eq!(eval_expr(&local, expression).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn resolved_closure_keeps_binding_selected_at_declaration() {
        let global = Env::new_global();
        Env::define(
            &global,
            "value".to_string(),
            Value::Str("global".to_string()),
        );
        Env::define(&global, "observed".to_string(), Value::Nil);
        let statement = resolve_statement(
            r#"{
                fun read() { observed = value; }
                read();
                var value = "block";
                read();
            }"#,
        )
        .unwrap();

        crate::rlox::eval::eval_statement(&global, statement).unwrap();

        assert_eq!(
            Env::get(&global, "observed"),
            Some(Value::Str("global".to_string()))
        );
    }

    #[test]
    fn local_reference_uses_current_scope() {
        let statement = resolve_statement("{ var local = 1; print local; }").unwrap();

        assert_eq!(
            references(&statement),
            vec![("local".to_string(), Local(0))]
        );
    }

    #[test]
    fn nested_reference_records_number_of_enclosing_scopes() {
        let statement = resolve_statement("{ var outer = 1; { { print outer; } } }").unwrap();

        assert_eq!(
            references(&statement),
            vec![("outer".to_string(), Local(2))]
        );
    }

    #[test]
    fn shadowing_selects_nearest_declared_binding() {
        let statement =
            resolve_statement("{ var value = 1; { print value; var value = 2; print value; } }")
                .unwrap();

        assert_eq!(
            references(&statement),
            vec![
                ("value".to_string(), Local(1)),
                ("value".to_string(), Local(0)),
            ]
        );
    }

    #[test]
    fn reference_before_later_local_declaration_resolves_to_outer_binding() {
        let statement =
            resolve_statement("{ { print value; var value = 2; print value; } }").unwrap();

        assert_eq!(
            references(&statement),
            vec![
                ("value".to_string(), Global),
                ("value".to_string(), Local(0)),
            ]
        );
    }

    #[test]
    fn leaving_nested_block_restores_enclosing_scope() {
        let statement = resolve_statement(
            "{ var expired = 0;  { var expired = 1; print expired; } print expired; }",
        )
        .unwrap();

        assert_eq!(
            references(&statement),
            vec![
                ("expired".to_string(), Local(0)),
                ("expired".to_string(), Local(0)),
            ]
        );
    }

    #[test]
    fn reading_local_variable_in_its_own_initializer_is_error() {
        let error = resolve_statement("{ var value = value; }").unwrap_err();

        assert!(matches!(
            error,
            RloxError::ResolverError(token) if token.lexeme == "value"
        ));
    }

    #[test]
    fn local_self_initializer_does_not_fall_through_to_outer_binding() {
        let error = resolve_statement("{ var value = 1; { var value = value; } }").unwrap_err();

        assert!(matches!(
            error,
            RloxError::ResolverError(token) if token.lexeme == "value"
        ));
    }

    #[test]
    fn global_self_initializer_is_resolved_as_global() {
        let statement = resolve_statement("var value = value;").unwrap();

        assert_eq!(references(&statement), vec![("value".to_string(), Global)]);
    }

    #[test]
    fn assignment_resolves_target_and_right_hand_side() {
        let statement =
            resolve_statement("{ var target = 0; var source = 1; target = source; }").unwrap();

        assert_eq!(
            references(&statement),
            vec![
                ("target".to_string(), Local(0)),
                ("source".to_string(), Local(0)),
            ]
        );
    }

    #[test]
    fn resolver_visits_all_compound_expression_children() {
        let statement = resolve_statement(
            "{ var a = 1; var b = 2; var c = 3; print !(a + b > 0) ? c : (a = b); }",
        )
        .unwrap();

        assert_eq!(
            references(&statement),
            vec![
                ("a".to_string(), Local(0)),
                ("b".to_string(), Local(0)),
                ("c".to_string(), Local(0)),
                ("a".to_string(), Local(0)),
                ("b".to_string(), Local(0)),
            ]
        );
    }

    #[test]
    fn resolver_visits_if_else_and_while_statements() {
        let statement = resolve_statement(
            "{ var condition = true; var item = 1;\
               if (condition) print item;\
               else while (condition) { print item; break; } }",
        )
        .unwrap();

        assert_eq!(
            references(&statement),
            vec![
                ("condition".to_string(), Local(0)),
                ("item".to_string(), Local(0)),
                ("condition".to_string(), Local(0)),
                ("item".to_string(), Local(1)),
            ]
        );
    }

    #[test]
    fn resolver_visits_callable_and_argument_expressions() {
        let statement =
            resolve_statement("{ var function; var argument; function(argument); }").unwrap();

        assert_eq!(
            references(&statement),
            vec![
                ("function".to_string(), Local(0)),
                ("argument".to_string(), Local(0)),
            ]
        );
    }

    #[test]
    fn named_function_parameters_and_locals_share_function_scope() {
        let statement = resolve_statement(
            "{ fun calculate(parameter) { var local = 1; return parameter + local; } }",
        )
        .unwrap();

        assert_eq!(
            references(&statement),
            vec![
                ("parameter".to_string(), Local(0)),
                ("local".to_string(), Local(0)),
            ]
        );
    }

    #[test]
    fn named_function_can_capture_enclosing_local() {
        let statement =
            resolve_statement("{ var captured = 1; fun read() { return captured; } }").unwrap();

        assert_eq!(
            references(&statement),
            vec![("captured".to_string(), Local(1))]
        );
    }

    #[test]
    fn named_function_can_resolve_itself_for_recursion() {
        let statement = resolve_statement("{ fun recurse() { return recurse; } }").unwrap();

        assert_eq!(
            references(&statement),
            vec![("recurse".to_string(), Local(1))]
        );
    }

    #[test]
    fn anonymous_function_resolves_parameters_and_captured_locals() {
        let statement = resolve_statement(
                "{ var captured = 1; var callable = fun (parameter) { return parameter + captured; }; }",
            )
                .unwrap();

        assert_eq!(
            references(&statement),
            vec![
                ("parameter".to_string(), Local(0)),
                ("captured".to_string(), Local(1)),
            ]
        );
    }

    #[test]
    fn resolved_named_function_reads_its_parameter_at_runtime() {
        let global = Env::new_global();
        let declaration = resolve_statement("fun identity(value) { return value; }").unwrap();
        crate::rlox::eval::eval_statement(&global, declaration).unwrap();
        let call = resolve_expression("identity(42)").unwrap();

        assert_eq!(eval_expr(&global, call).unwrap(), Value::Number(42.0));
    }

    #[test]
    fn duplicate_local_declaration_is_error() {
        let error = resolve_statement("{ var value = 1; var value = 2; }").unwrap_err();

        assert!(matches!(
            error,
            RloxError::ResolverError(token) if token.lexeme == "value"
        ));
    }

    #[test]
    fn resolver_discards_scope_even_when_resolution_fails() {
        let mut resolver = Resolver::new();
        let mut invalid = parse_statement("{ var leaked = 1; var bad = bad; }");
        assert!(resolver.resolve_statement(&mut invalid).is_err());

        let mut expression = parse_expression("leaked");
        resolver.resolve_expression(&mut expression).unwrap();

        assert_eq!(expression_resolution(&expression), &Global);
    }

    #[test]
    fn resolved_statement_still_evaluates_local_hops() {
        let global = Env::new_global();
        Env::define(&global, "observed".to_string(), Value::Nil);
        let statement = resolve_statement("{ var value = 42; { observed = value; } }").unwrap();

        assert!(matches!(
            crate::rlox::eval::eval_statement(&global, statement).unwrap(),
            ExecFlow::Normal(None)
        ));
        assert_eq!(Env::get(&global, "observed"), Some(Value::Number(42.0)));
    }
}
