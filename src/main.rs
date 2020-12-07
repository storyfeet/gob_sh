mod args;
mod exec;
mod parser;
mod settings;
mod statement;
use gobble::traits::*;

//use std::env;
use std::io::*;
//use std::path::Path;
//use std::process::*;

fn main() {
    let mut sets = settings::Settings::new();
    loop {
        print!("> ");
        stdout().flush().ok();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        let statement = match parser::FullStatement.parse_s(&input) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", e);
                continue;
            }
        };
        match statement.run(&mut sets) {
            Ok(true) => println!("\nOK - Success"),
            Ok(false) => println!("\nOK - fail"),
            Err(e) => println!("\nErr - {}", e),
        }
    }
}
