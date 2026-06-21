use std::fs::File;
use std::io;
use std::io::{Read, Write};
pub struct RLox {
    pub had_error: bool,
}

impl RLox {
    pub fn run_file(&self, path: &str) -> anyhow::Result<()> {
        let mut file = File::open(path)?;
        let mut result = String::new();
        file.read_to_string(&mut result)?;
        self.run(&result)
    }

    pub fn run_prompt(&mut self) -> anyhow::Result<()> {
        let mut line = String::new();
        let stdin = io::stdin();
        let mut stdout = io::stdout();
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

    pub fn run(&self, code: &str) -> anyhow::Result<()> {
        for s in code.chars() {
            print!("{s}")
        }
        Ok(())
    }

    pub fn error(&mut self, line: u64, message: &str) {
        self.had_error = true;
        self.report(line, "", message)
    }

    pub fn report(&self, line: u64, where_location: &str, message: &str) {
        eprintln!("[line {line}] Error {where_location}: {message}")
    }
}
