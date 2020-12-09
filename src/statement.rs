use crate::args::{Arg, Args};
use crate::channel::Channel;
use crate::exec::Exec;
use crate::settings::{Data, Settings};
use err_tools::*;
use std::process::Stdio;

pub enum Statement {
    Exec(Exec),
    Write {
        exec: Exec,
        chan: Channel,
        filename: Arg,
        append: bool,
    },
    Let(Vec<String>, Args),
}

impl Statement {
    pub fn run(&self, s: &mut Settings) -> anyhow::Result<bool> {
        match self {
            Statement::Exec(e) => {
                let mut ch = e.run(s, Stdio::inherit(), Stdio::inherit(), Stdio::inherit())?;
                ch.wait().map(|e| e.success()).map_err(Into::into)
            }

            Statement::Write {
                exec,
                chan,
                filename,
                append,
            } => {
                let filename = filename.run(s)?.to_string();
                let ch = exec.run(s, Stdio::inherit(), Stdio::piped(), Stdio::piped())?;
                let mut iread =
                    chan.as_reader(ch.stdout.e_str("No Output")?, ch.stderr.e_str("No ErrPut")?);

                let mut f = std::fs::OpenOptions::new()
                    .append(*append)
                    .truncate(!*append)
                    .write(true)
                    .create(true)
                    .open(filename)?;
                std::io::copy(&mut iread, &mut f)
                    .map(|_| true)
                    .e_str("Could not copy to output file")
            }
            Statement::Let(names, args) => {
                let ag = args.run(s)?;
                if ag.len() < names.len() {
                    return e_str("Not enough results for var names");
                }
                for (n, k) in names.iter().enumerate() {
                    s.set(k.to_string(), Data::Str(ag[n].clone()))
                }
                Ok(true)
            }
        }
    }
}
