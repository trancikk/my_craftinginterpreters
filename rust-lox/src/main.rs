use anyhow::{Error, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::{env, io, process};

fn main() {
    let args = env::args();
    let len = args.len();
    let result = match len {
        v if v > 2 => {
            println!("Usage: rlox [script]");
            process::exit(64);
        }
        v if v == 2 => run_file(&env::args().skip(1).next().unwrap()),
        _ => run_prompt(),
    };
    result.unwrap()
}

fn run_file(path: &str) -> Result<()> {
    let mut file = File::open(path)?;
    let mut result = String::new();
    file.read_to_string(&mut result)?;
    run(&result)
}

fn run_prompt() -> Result<()> {
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
                run(&line)?;
                line.clear()
            } else {
                break;
            }
        }
    }
    Ok(())
}

fn run(code: &str) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
