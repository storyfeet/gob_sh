use crate::exec::Exec;
use crate::settings::{Data, Settings};
use err_tools::*;
use std::io::Read;
use std::process::Stdio;

pub struct Args(pub Vec<Arg>);

fn try_glob(s: &str, args: &mut Vec<String>) {
    match glob::glob(s) {
        Ok(v) => {
            let mut found = false;
            for a in v {
                found = true;
                if let Ok(val) = a {
                    args.push(val.display().to_string());
                }
            }
            if !found {
                args.push(s.to_string());
            }
        }
        Err(_) => {
            args.push(s.to_string());
        }
    }
}

impl Args {
    pub fn run(&self, sets: &mut Settings) -> anyhow::Result<Vec<String>> {
        let mut res = Vec::new();
        for a in &self.0 {
            match a.run(sets)? {
                Data::RawStr(s) => res.push(s),
                Data::Str(s) => try_glob(&s, &mut res),
                Data::List(l) => {
                    for v in l {
                        res.push(v.to_string());
                    }
                }
                v => res.push(v.to_string()),
            }
        }
        Ok(res)
    }
}

pub enum Arg {
    RawString(String),
    StringLit(String),
    StringExpr(Vec<Arg>),
    Var(String),
    Command(Exec),
}

impl Arg {
    pub fn run(&self, sets: &mut Settings) -> anyhow::Result<Data> {
        match self {
            Arg::RawString(s) => Ok(Data::RawStr(s.to_string())),
            Arg::StringLit(s) => Ok(Data::Str(s.to_string())),
            Arg::StringExpr(v) => {
                let mut s = String::new();
                for a in v {
                    s.push_str(&a.run(sets)?.to_string());
                }
                Ok(Data::Str(s.to_string()))
            }
            Arg::Var(name) => sets.get(name).e_str("No Var by that name"),
            Arg::Command(ex) => {
                let ch = ex.run(sets, Stdio::null(), Stdio::piped(), Stdio::inherit())?;
                let mut buf = String::new();
                ch.stdout
                    .e_str("No Return Buffer")?
                    .read_to_string(&mut buf);

                //ch.wait(); TODO work out if this is needed
                Ok(Data::RawStr(buf))
            }
        }
    }
}
