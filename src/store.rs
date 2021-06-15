use crate::parser;
use bogobble::traits::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::io::Read;
use std::path::Path;
use std::rc::Rc;

/*#[derive(Debug, Clone)]
pub struct Store {
    scopes: Vec<BTreeMap<String, Data>>,
}*/

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

#[derive(Debug, Clone)]
pub struct Store(Rc<RefCell<IStore>>);

#[derive(Debug, Clone)]
pub struct IStore {
    data: BTreeMap<String, Data>,
    //f_top: bool,
    parent: Option<Rc<RefCell<IStore>>>,
}

impl IStore {
    fn get(&self, k: &str) -> Option<Data> {
        match self.data.get(k) {
            Some(v) => Some(v.clone()),
            None => match self.parent {
                Some(ref v) => v.borrow().get(k),
                None => std::env::var(k).ok().map(Data::Str),
            },
        }
    }

    /// None returned means, data was added
    fn set(&mut self, k: &str, v: Data) -> Option<Data> {
        match self.data.get_mut(k) {
            Some(a) => {
                *a = v;
                None
            }
            None => match self.parent {
                Some(ref p) => p.borrow_mut().set(k, v),
                None => Some(v),
            },
        }
    }

    fn for_each<F: FnMut(&str, &Data)>(&self, f: &mut F) {
        if let Some(p) = &self.parent {
            p.borrow().for_each(f);
        }
        for (k, v) in &self.data {
            f(k, v)
        }
    }

    fn scope_depth(&self) -> usize {
        match &self.parent {
            Some(p) => p.borrow().scope_depth() + 1,
            None => 0,
        }
    }
}

impl Store {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(IStore {
            data: BTreeMap::new(),
            parent: None,
        })))
    }
    pub fn get(&self, k: &str) -> Option<Data> {
        self.0.borrow().get(k)
    }

    pub fn let_set(&self, k: String, v: Data) {
        self.0.borrow_mut().data.insert(k, v);
    }

    pub fn push(&mut self) {
        *self = self.child();
    }
    pub fn child(&self) -> Self {
        Self(Rc::new(RefCell::new(IStore {
            data: BTreeMap::new(),
            parent: Some(self.0.clone()),
        })))
    }

    fn parent(&self) -> Self {
        match self.0.borrow().parent {
            Some(ref p) => Self(p.clone()),
            None => Self::new(),
        }
    }
    pub fn pop(&mut self) {
        *self = self.parent();
    }

    pub fn scope_depth(&mut self) -> usize {
        self.0.borrow().scope_depth()
    }

    pub fn set(&mut self, k: String, v: Data) {
        let mut m = self.0.borrow_mut();
        let v = m.set(&k, v);
        drop(m);
        match v {
            Some(v) => self.let_set(k, v),
            None => {}
        }
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

    pub fn for_each<F: FnMut(&str, &Data)>(&self, mut f: F) {
        self.0.borrow().for_each(&mut f)
    }

    pub fn as_map(&self) -> BTreeMap<String, Data> {
        let mut mp = BTreeMap::new();
        self.for_each(|k, v| {
            mp.insert(k.to_string(), v.clone());
        });
        mp
    }
}

/*impl Store {
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
}*/
