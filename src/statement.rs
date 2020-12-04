use err_tools::*;
use std::process::{Child, Command, Stdio};

#[derive(Clone)]
pub enum Join {
    Pipe,
    PipeErr,
}

pub enum Exec {
    Join(Join, Box<Exec>, Box<Exec>),
    Simple(String, Vec<String>),
}

pub enum Statement {
    Exec(Exec),
}

impl Statement {
    pub fn run(&self, s: &mut Settings) {}
}

/// Some options for statements to run, or persistent data
pub struct Settings {}

impl Exec {
    pub fn run(&self, s: &mut Settings, input: Stdio, output: Stdio) -> anyhow::Result<Child> {
        match self {
            Exec::Simple(name, args) => Command::new(name)
                .args(args)
                .stdin(input)
                .stdout(output)
                .spawn()
                .map_err(Into::into),
            Exec::Join(Join::Pipe, a, b) => {
                let ch = a.run(s, input, Stdio::piped())?;
                b.run(
                    s,
                    Stdio::from(ch.stdout.e_str("No Output from job 'A'")?),
                    output,
                )
            }
            Statement::Join(Join::Or, a, b) => match a.run(s, input, output) {
                Ok(ch) => Ok(ch),
                Err(_) => b.run(s, input, output),
            },
        }
    }
}
