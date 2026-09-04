use crate::rlox::eval::{RuntimeError, eval_expr};
use crate::rlox::parser::{Expr, Parser};
use crate::rlox::scanner::Scanner;
use crate::rlox::token::Token;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::process::exit;
use thiserror::Error;

pub struct RLox {
    pub had_error: bool,
}

// TODO add RuntimeError, use it in report error method
#[derive(Error, Debug)]
pub enum RloxError {
    #[error("Unexpected token `{0:?}`!")]
    ParserError(Token, String),
    #[error("Unexpected Symbols `{1:?}` on line: `{0:?}`!")]
    ScannerError(u64, char, String),
}

impl RLox {
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
            let expr = parser.expression();
            match expr {
                None => {
                    return;
                }
                Some(expr) => {
                    self.interpret(expr);
                }
            }
        }
        {}
        // let _: Vec<Token> = scanner.collect(self);
    }

    pub fn error(&mut self, line: u64, message: &str) {
        self.had_error = true;
        self.report(line, "", message);
    }

    pub fn report(&self, line: u64, where_location: &str, message: &str) {
        eprintln!("[line {line}] Error {where_location}: {message}");
    }

    pub fn report_token(&mut self, token: &Token, msg: &str) {
        self.had_error = true;
        eprintln!("{:?} at '{:?}' {:?}", token.line, token.lexeme, msg);
    }

    pub fn interpret(&mut self, expr: Expr) {
        let result = eval_expr(expr);
        match result {
            Ok(value) => println!("{:?}", value),
            Err(e) => match e {
                RuntimeError::IncorrectType(_, token) => {
                    self.report_token(&token, "Error during expression evaluation");
                }
                RuntimeError::IncorrectOperator(token) => {
                    self.report_token(&token, "Error during expression evaluation")
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut rlox = RLox { had_error: false };
        let result = rlox.run("1+2");
        assert!(!rlox.had_error);
    }
}
