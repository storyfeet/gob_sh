//! Some options for statements to run, or persistent data

use crate::ui;
use crate::RT;
use std::collections::BTreeMap;
use std::fmt::{self, Display};
//use std::io::Write;
//use termion::{clear, cursor, cursor::DetectCursorPos};

#[derive(Clone, Debug)]
pub enum Data {
    Bool(bool),
    Str(String),
    RawStr(String),
    List(Vec<Data>),
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
        }
    }
}

impl Data {
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

pub struct Settings {
    pub line: String,
    scopes: Vec<BTreeMap<String, Data>>,
}

impl Settings {
    /// Invariants : Settings must always have at least one layer in scope.
    pub fn new() -> Settings {
        Settings {
            line: String::new(),
            scopes: vec![BTreeMap::new()],
        }
    }

    pub fn add_char(&mut self, c: char, rt: &mut RT) {
        ui::unprint(&self.line, rt);
        self.line.push(c);
        ui::print(&self.line, rt);
    }

    pub fn del_char(&mut self, rt: &mut RT) {
        ui::unprint(&self.line, rt);
        ui::del_char(&mut self.line);
        ui::print(&self.line, rt);
    }

    pub fn del_line(&mut self, rt: &mut RT) {
        ui::unprint(&self.line, rt);
        ui::del_line(&mut self.line);
        ui::print(&self.line, rt);
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
}
