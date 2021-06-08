use crate::exec::Exec;
use crate::store::{Data, Store};
use err_tools::*;
use std::collections::BTreeMap;
use std::io::Read;
use std::process::Stdio;

#[derive(Clone, Debug)]
pub struct Args(pub Vec<Arg>);

fn try_glob<F: FnMut(String)>(s: &str, mut push: F) {
    match glob::glob(s) {
        Ok(v) => {
            let mut found = false;
            for a in v {
                found = true;
                if let Ok(val) = a {
                    push(val.display().to_string());
                }
            }
            if !found {
                push(s.to_string());
            }
        }
        Err(_) => {
            push(s.to_string());
        }
    }
}

impl Args {
    pub fn run(&self, sets: &mut Store) -> anyhow::Result<Vec<String>> {
        let mut res = Vec::new();
        for a in &self.0 {
            match a.run(sets)? {
                Data::RawStr(s) => res.push(s),
                Data::Str(s) => try_glob(&s, |d| res.push(d)),
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

    pub fn run_data(&self, sets: &mut Store) -> anyhow::Result<Vec<Data>> {
        let mut res = Vec::new();
        for a in &self.0 {
            res.push(a.run(sets)?);
        }
        Ok(res)
    }
}

#[derive(Debug, Clone)]
pub enum Arg {
    RawString(String),
    StringLit(String),
    HomePath(String),
    HomeExpr(Vec<Arg>),
    StringExpr(Vec<Arg>),
    Var(String),
    List(Args),
    Map(Vec<(String, Arg)>),
    Command(Exec),
    ArrCommand(Exec),
}

impl Arg {
    pub fn run(&self, sets: &mut Store) -> anyhow::Result<Data> {
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
            Arg::HomeExpr(v) => {
                let mut hp = std::env::var("HOME").unwrap_or(String::new());
                for a in v {
                    hp.push_str(&a.run(sets)?.to_string());
                }
                Ok(Data::Str(hp))
            }
            Arg::HomePath(s) => {
                let hp = std::env::var("HOME").unwrap_or(String::new());
                Ok(Data::Str(format!("{}{}", hp, s)))
            }
            Arg::Var(name) => sets.get(name).e_str("No Var by that name"),
            Arg::Command(ex) => {
                let ch = ex.run(sets, Stdio::null(), Stdio::piped(), Stdio::inherit())?;
                let mut buf = String::new();
                ch.stdout
                    .e_str("No Return Buffer")?
                    .read_to_string(&mut buf)
                    .ok();

                //ch.wait(); TODO work out if this is needed
                Ok(Data::RawStr(buf))
            }
            Arg::ArrCommand(ex) => {
                let ch = ex.run(sets, Stdio::null(), Stdio::piped(), Stdio::inherit())?;
                let mut buf = String::new();
                ch.stdout
                    .e_str("No Return Buffer")?
                    .read_to_string(&mut buf)
                    .ok();

                let v: Vec<Data> = buf
                    .split(|c| " \r\t\n".contains(c))
                    .filter(|a| a.len() > 0)
                    .map(|a| Data::RawStr(a.to_string()))
                    .collect();
                //ch.wait(); TODO work out if this is needed
                Ok(Data::List(v))
            }
            Arg::List(l) => {
                let mut res = Vec::new();
                for c in l.run_data(sets)? {
                    match c {
                        Data::Str(s) => try_glob(&s, |d| res.push(Data::Str(d))),
                        o => res.push(o),
                    }
                }

                Ok(Data::List(res))
            }
            Arg::Map(mp) => {
                let mut res = BTreeMap::new();
                for (k, v) in mp {
                    res.insert(k.to_string(), v.run(sets)?);
                }
                Ok(Data::Map(res))
            }
        }
    }
}
