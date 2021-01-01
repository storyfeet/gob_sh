use crate::args::{Arg, Args};
use crate::channel::Channel;
use crate::exec::Exec;
use crate::store::{Data, Store};
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
    Export(Vec<String>, Args),
    Cd(Arg),
}

impl Statement {
    pub fn run(&self, s: &mut Store) -> anyhow::Result<bool> {
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
            Statement::Export(names, args) => {
                let ag = args.run(s)?;
                if ag.len() < names.len() {
                    return e_str("Not enough results for var names");
                }
                for (n, k) in names.iter().enumerate() {
                    std::env::set_var(k.to_string(), ag[n].to_string());
                    //                    s.set(k.to_string(), Data::Str(ag[n].clone()))
                }
                Ok(true)
            }
            Statement::Cd(arg) => {
                let mut run_res = arg.run(s)?.to_string();
                if let Some('~') = run_res.chars().next() {
                    let hm = std::env::var("HOME")?;
                    run_res = run_res.replace('~', &hm);
                }

                let s2 = glob::glob(&run_res)?
                    .next()
                    .e_str("Could not glob arg for cd")??;

                std::env::set_current_dir(s2)?;

                std::env::set_var("PWD", std::env::current_dir()?);
                Ok(true)
            }
        }
    }
}
