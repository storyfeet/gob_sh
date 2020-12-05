use crate::exec::Exec;
//use err_tools::*;
use std::process::Stdio;

pub enum Statement {
    Exec(Exec),
}

impl Statement {
    pub fn run(&self, s: &mut Settings) -> anyhow::Result<bool> {
        match self {
            Statement::Exec(e) => {
                let mut ch = e.run(s, Stdio::inherit(), Stdio::inherit(), Stdio::inherit())?;
                ch.wait().map(|e| e.success()).map_err(Into::into)
            }
        }
    }
}

/// Some options for statements to run, or persistent data
pub struct Settings {}
