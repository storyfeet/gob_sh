mod args;
mod exec;
mod parser;
mod statement;
use gobble::traits::*;

//use std::env;
use std::io::*;
//use std::path::Path;
//use std::process::*;

fn main() {
    loop {
        print!("> ");
        stdout().flush().ok();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        let statement = match parser::Statement.parse_s(&input) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", e);
                continue;
            }
        };
        match statement.run(&mut statement::Settings {}) {
            Ok(true) => println!("\nOK - Success"),
            Ok(false) => println!("\nOK - fail"),
            Err(e) => println!("\nErr - {}", e),
        }
    }
}
