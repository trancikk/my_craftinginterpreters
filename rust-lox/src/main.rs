mod rlox;

use crate::rlox::interpreter::RLox;
use std::{env, process};

fn main() {
    let args = env::args();
    let len = args.len();
    let mut interpreter = RLox::new();
    let result = match len {
        v if v > 2 => {
            println!("Usage: rlo [script]");
            process::exit(64);
        }
        v if v == 2 => interpreter.run_file(&env::args().nth(1).unwrap()),
        _ => interpreter.run_prompt(),
    };
    result.unwrap();
    if interpreter.had_error {
        process::exit(65);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
