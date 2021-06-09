use crate::parser;
use bogobble::traits::*;
use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Store {
    scopes: Vec<BTreeMap<String, Data>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Data {
    Bool(bool),
    Str(String),
    RawStr(String),
    List(Vec<Data>),
    Map(BTreeMap<String, Data>),
}

impl Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Data::Bool(true) => write!(f, "true"),
            Data::Bool(false) => write!(f, "false"),
            Data::Str(s) | Data::RawStr(s) => write!(f, "{}", s),
            Data::List(l) => {
                write!(f, "[ ").ok();
                for (n, v) in l.iter().enumerate() {
                    if n > 0 {
                        write!(f, ", ").ok();
                    }
                    write!(f, "{}", v).ok();
                }
                write!(f, "]")
            }
            Data::Map(m) => {
                write!(f, "{{").ok();
                let mut join = "";
                for (k, v) in m {
                    write!(f, "{}{}:{}", join, k, v).ok();
                    join = ", ";
                }

                write!(f, "}}")
            }
        }
    }
}

impl Data {
    #[allow(dead_code)]
    fn on_args(self, vec: &mut Vec<String>) {
        match self {
            Data::List(l) => {
                for val in l {
                    vec.push(val.to_string());
                }
            }
            d => vec.push(d.to_string()),
        }
    }
}

impl Store {
    pub fn new() -> Self {
        Self {
            scopes: vec![BTreeMap::new()],
        }
    }
    pub fn get(&self, k: &str) -> Option<Data> {
        let l = self.scopes.len();
        for i in 0..l {
            let n = (l - 1) - i;
            if let Some(ov) = self.scopes[n].get(k) {
                return Some(ov.clone());
            }
        }

        std::env::var(k).ok().map(Data::Str)
    }

    pub fn let_set(&mut self, k: String, v: Data) {
        let last = self.scopes.len() - 1;
        self.scopes[last].insert(k, v);
    }

    pub fn push(&mut self) {
        self.scopes.push(BTreeMap::new());
    }
    pub fn pop(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }
    pub fn set(&mut self, k: String, v: Data) {
        let l = self.scopes.len();
        for i in 0..l {
            let n = (l - 1) - i;
            if let Some(ov) = self.scopes[n].get_mut(&k) {
                *ov = v;
                return;
            }
        }

        self.let_set(k, v)
    }
    pub fn source_path<P: AsRef<Path>>(&mut self, p: P) -> anyhow::Result<()> {
        let mut f = std::fs::File::open(p)?;
        let mut buf = String::new();
        f.read_to_string(&mut buf)?;
        let p = parser::Lines.parse_s(&buf).map_err(|e| e.strung())?;
        for v in p {
            v.run(self).ok();
        }
        Ok(())
    }
}
