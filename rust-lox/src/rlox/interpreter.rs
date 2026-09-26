use crate::rlox::environment::{Env, EnvRef};
use crate::rlox::eval::{RuntimeError, eval_statement};
use crate::rlox::parser::{Parser, Statement};
use crate::rlox::scanner::Scanner;
use crate::rlox::token::Token;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::process::exit;
use thiserror::Error;

pub struct RLox {
    pub had_error: bool,
    env: EnvRef,
}

// TODO add RuntimeError, use it in report error method
#[derive(Error, Debug)]
pub enum RloxError {
    #[error("Unexpected token `{0:?}`!")]
    ParserError(Token, String),
    #[error("Unexpected Symbols `{1:?}` on line: `{0:?}`!")]
    ScannerError(u64, char, String),
    #[error("Got EOF")]
    EOF,
}

impl RLox {
    pub fn new() -> RLox {
        RLox {
            env: Env::new(),
            had_error: false,
        }
    }

    pub fn run_file(&mut self, path: &str) -> anyhow::Result<()> {
        // TODO make is return lines
        let mut file = File::open(path)?;
        let mut result = String::new();
        file.read_to_string(&mut result)?;
        self.run(&result);
        if self.had_error {
            exit(1);
        }
        Ok(())
    }

    pub fn run_prompt(&mut self) -> anyhow::Result<()> {
        let mut line = String::new();
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        // TODO return lines
        loop {
            let mut lock = stdout.lock();
            write!(lock, "> ")?;
            stdout.flush()?;
            let count = stdin.read_line(&mut line);
            if let Ok(count) = count {
                if count != 0 {
                    self.run(&line);
                    line.clear();
                    self.had_error = false;
                } else {
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn run(&mut self, code: &str) {
        let lines = code.lines();
        let scanner = Scanner::new(lines);
        let mut parser = Parser::new(scanner);
        loop {
            let stmtns = parser.parse_statements();
            match stmtns {
                Err(e) => {
                    let msg = e.to_string();
                    match e {
                        RloxError::EOF => break,
                        _ => eprintln!("{msg}"),
                    }
                    // match e {
                    //
                    // }
                    eprintln!("{e}");
                }
                Ok(expr) => {
                    self.interpret(expr);
                }
            }
        }
    }

    // pub fn error(&mut self, line: u64, message: &str) {
    //     self.had_error = true;
    //     self.report(line, "", message);
    // }

    // pub fn report(&self, line: u64, where_location: &str, message: &str) {
    //     eprintln!("[line {line}] Error {where_location}: {message}");
    // }

    pub fn report_token(&mut self, token: &Token, msg: &str) {
        self.had_error = true;
        eprintln!("{:?} at '{:?}' {:?}", token.line, token.lexeme, msg);
    }

    pub fn interpret(&mut self, statement: Statement) {
        let result = eval_statement(&self.env, statement);
        match result {
            // Ok(value) => println!("{value:?}"),
            Ok(_) => {}
            Err(e) => {
                let e_msg = e.to_string();
                let token = match e {
                    RuntimeError::IncorrectType(_, token) => token,
                    RuntimeError::IncorrectOperator(token) => token,
                    RuntimeError::EvaluationError(_, token) => token,
                    RuntimeError::IdentifierNotFound(token) => token,
                };
                self.report_token(&token, &e_msg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut rlox = RLox::new();
        rlox.run("1+2");
        assert!(!rlox.had_error);
    }

    #[test]
    fn var_declaration_and_lookup() {
        let mut rlox = RLox::new();
        rlox.run("var x = 42; x;");
        assert!(!rlox.had_error);
    }

    #[test]
    fn var_declaration_without_initializer() {
        let mut rlox = RLox::new();
        rlox.run("var x; x;");
        assert!(!rlox.had_error);
    }

    #[test]
    fn multiple_variables() {
        let mut rlox = RLox::new();
        rlox.run("var a = 1; var b = 2; a + b;");
        assert!(!rlox.had_error);
    }

    #[test]
    fn variable_in_expression() {
        let mut rlox = RLox::new();
        rlox.run("var x = 10; x * 2 + 5;");
        assert!(!rlox.had_error);
    }

    #[test]
    fn undefined_variable_error() {
        let mut rlox = RLox::new();
        rlox.run("undefined_var;");
        assert!(rlox.had_error);
    }

    #[test]
    fn redefine_variable() {
        let mut rlox = RLox::new();
        rlox.run("var x = 1; var x = 2; x;");
        assert!(!rlox.had_error);
    }

    #[test]
    fn variable_in_comparison() {
        let mut rlox = RLox::new();
        rlox.run("var x = 5; x > 3;");
        assert!(!rlox.had_error);
    }

    #[test]
    fn variable_in_ternary() {
        let mut rlox = RLox::new();
        rlox.run("var cond = true; cond ? 1 : 2;");
        assert!(!rlox.had_error);
    }

    #[test]
    fn print_variable() {
        let mut rlox = RLox::new();
        rlox.run("var x = 42; print x;");
        assert!(!rlox.had_error);
    }
}
