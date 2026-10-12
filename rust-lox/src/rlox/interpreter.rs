use crate::rlox::environment::{Env, EnvRef};
use crate::rlox::eval::{RuntimeError, eval_statement};
use crate::rlox::natives::define_clock;
use crate::rlox::parser::ExecFlow::Normal;
use crate::rlox::parser::{Parser, ParserMode, Statement};
use crate::rlox::scanner::Scanner;
use crate::rlox::token::Token;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::process::exit;
use thiserror::Error;

pub struct RLox {
    pub had_error: bool,
    global_env: EnvRef,
}

// TODO add RuntimeError, use it in report error method
#[derive(Error, Debug)]
pub enum RloxError {
    #[error("Parser Error: Unexpected token `{0:?}`!")]
    ParserError(Token, String),
    #[error("Scanner Error: Unexpected Symbols `{1:?}` on line: `{0:?}`!")]
    ScannerError(u64, char, String),
    #[error("Got EOF")]
    EOF,
    #[error("Resolver Error: Unexpected Token `{0:?}`")]
    ResolverError(Token)
}

impl RLox {
    pub fn new() -> RLox {
        let env = Env::new();
        define_clock(env.clone());
        RLox {
            global_env: env,
            had_error: false,
        }
    }

    pub fn run_file(&mut self, path: &str) -> anyhow::Result<()> {
        // TODO make is return lines
        let mut file = File::open(path)?;
        let mut result = String::new();
        file.read_to_string(&mut result)?;
        self.run(&result, ParserMode::Script);
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
                    self.run(&line, ParserMode::Repl);
                    line.clear();
                    self.had_error = false;
                } else {
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn run(&mut self, code: &str, parser_mode: ParserMode) {
        let lines = code.lines();
        let scanner = Scanner::new(lines);
        let mut parser = Parser::new_with_mode(scanner, parser_mode);
        loop {
            let stmtns = parser.parse_statements();
            match stmtns {
                Err(e) => {
                    let msg = e.to_string();
                    match e {
                        RloxError::EOF => break,
                        // RloxError::ParserError(token, msg) => {
                        //     eprintln!("{e} - {msg}")
                        // }
                        _ => eprintln!("{msg}"),
                    }
                    // match e {
                    //
                    // }
                    // eprintln!("{e}");
                }
                Ok(stmnt) => {
                    self.interpret(stmnt, parser_mode);
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
        eprintln!("{:?} at '{:?}' {:?}", msg, token.line, token.lexeme);
    }

    pub fn interpret(&mut self, statement: Statement, parser_mode: ParserMode) {
        let result = eval_statement(&self.global_env, statement);
        match result {
            Ok(value) => {
                if let Normal(Some(value)) = value
                    && parser_mode == ParserMode::Repl
                {
                    println!("{}", value)
                }
            }
            // Ok(_) => {}
            Err(e) => {
                let e_msg = e.to_string();
                let token = match e {
                    RuntimeError::IncorrectType(_, token) => token,
                    RuntimeError::IncorrectOperator(token) => token,
                    RuntimeError::EvaluationError(_, token) => token,
                    RuntimeError::IdentifierNotFound(token) => token,
                    RuntimeError::IdentifierNotInitialized(token) => token,
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
        rlox.run("1+2", ParserMode::Repl);
        assert!(!rlox.had_error);
    }

    #[test]
    fn var_declaration_and_lookup() {
        let mut rlox = RLox::new();
        rlox.run("var x = 42; x;", ParserMode::Repl);
        assert!(!rlox.had_error);
    }

    #[test]
    fn reading_variable_without_initializer_is_runtime_error() {
        let mut rlox = RLox::new();
        rlox.run("var x; x;", ParserMode::Repl);
        assert!(rlox.had_error);
    }

    #[test]
    fn assignment_initializes_variable() {
        let mut rlox = RLox::new();
        rlox.run("var x; x = 42; x;", ParserMode::Repl);
        assert!(!rlox.had_error);
    }

    #[test]
    fn nil_is_valid_initialized_value() {
        let mut rlox = RLox::new();
        rlox.run("var x = nil; x;", ParserMode::Repl);
        assert!(!rlox.had_error);
    }

    #[test]
    fn multiple_variables() {
        let mut rlox = RLox::new();
        rlox.run("var a = 1; var b = 2; a + b;", ParserMode::Repl);
        assert!(!rlox.had_error);
    }

    #[test]
    fn variable_in_expression() {
        let mut rlox = RLox::new();
        rlox.run("var x = 10; x * 2 + 5;", ParserMode::Repl);
        assert!(!rlox.had_error);
    }

    #[test]
    fn undefined_variable_error() {
        let mut rlox = RLox::new();
        rlox.run("undefined_var;", ParserMode::Repl);
        assert!(rlox.had_error);
    }

    #[test]
    fn redefine_variable() {
        let mut rlox = RLox::new();
        rlox.run("var x = 1; var x = 2; x;", ParserMode::Repl);
        assert!(!rlox.had_error);
    }

    #[test]
    fn variable_in_comparison() {
        let mut rlox = RLox::new();
        rlox.run("var x = 5; x > 3;", ParserMode::Repl);
        assert!(!rlox.had_error);
    }

    #[test]
    fn variable_in_ternary() {
        let mut rlox = RLox::new();
        rlox.run("var cond = true; cond ? 1 : 2;", ParserMode::Repl);
        assert!(!rlox.had_error);
    }

    #[test]
    fn print_variable() {
        let mut rlox = RLox::new();
        rlox.run("var x = 42; print x;", ParserMode::Repl);
        assert!(!rlox.had_error);
    }
}
