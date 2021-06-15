use err_tools::*;
use std::collections::BTreeMap;
use std::fmt::{self, Display};
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

    pub fn push(&mut self, b: Self) -> anyhow::Result<()> {
        match (self, b) {
            (Data::List(a), Data::List(b)) => a.extend(b),
            (Data::List(a), b) => a.push(b),
            (Data::Str(a), b) => a.push_str(&b.to_string()),
            (Data::Map(a), Data::Map(b)) => a.extend(b),
            (a, b) => return e_string(format!("could not push {:?} onto {:?}", b, a)),
        }
        Ok(())
    }
}
