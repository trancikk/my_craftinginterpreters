use crate::rlox::parser::Parser;
use crate::rlox::scanner::Scanner;
use crate::rlox::token::Token;
use std::fs::File;
use std::io;
use std::io::{Read, Write};

pub struct RLox {
    pub had_error: bool,
}

impl RLox {
    pub fn run_file(&mut self, path: &str) -> anyhow::Result<()> {
        // TODO make is return lines
        let mut file = File::open(path)?;
        let mut result = String::new();
        file.read_to_string(&mut result)?;
        self.run(&result)
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
                    self.run(&line)?;
                    line.clear();
                    self.had_error = false
                } else {
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn run(&mut self, code: &str) -> anyhow::Result<()> {
        let lines = code.lines();
        let scanner = Scanner::new(lines);
        let mut parser = Parser::new(scanner, self);
        parser.parse();
        // let _: Vec<Token> = scanner.collect(self);
        Ok(())
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut rlox = RLox { had_error: false };
        let result = rlox.run("1+2");
        assert!(result.is_ok());
        assert!(!rlox.had_error);
    }
}
