use crate::data::*;
use crate::parser;
use bogobble::traits::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::rc::Rc;

/*#[derive(Debug, Clone)]
pub struct Store {
    scopes: Vec<BTreeMap<String, Data>>,
}*/

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

    /// None returned means, data was added
    fn push_set(&mut self, k: &str, v: Data) -> anyhow::Result<Option<Data>> {
        match self.data.get_mut(k) {
            Some(a) => {
                a.push(v)?;
                Ok(None)
            }
            None => match self.parent {
                Some(ref p) => p.borrow_mut().push_set(k, v),
                None => {
                    //try pushing on Env var
                    match std::env::var(k) {
                        Ok(mut e) => {
                            e.push_str(&v.to_string());
                            std::env::set_var(k, e);
                            Ok(None)
                        }
                        Err(_) => Ok(Some(v)),
                    }
                }
            },
        }
    }

    fn for_each<F: FnMut(&str, &Data, usize)>(&self, f: &mut F) {
        let d = self.scope_depth();
        if let Some(p) = &self.parent {
            p.borrow().for_each(f);
        }
        for (k, v) in &self.data {
            f(k, v, d)
        }
    }

    fn do_with<F: FnOnce(&Data)>(&self, k: &str, f: F) {
        match self.data.get(k) {
            Some(v) => f(v),
            None => match &self.parent {
                Some(p) => p.borrow().do_with(k, f),
                None => {}
            },
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

    pub fn do_with<F: FnOnce(&Data)>(&self, k: &str, f: F) {
        self.0.borrow().do_with(k, f)
    }

    pub fn let_set(&self, k: String, v: Data) {
        self.0.borrow_mut().data.insert(k, v);
    }

    pub fn push_set(&self, k: String, v: Data) -> anyhow::Result<()> {
        let mut m = self.0.borrow_mut();
        let v = m.push_set(&k, v);
        drop(m);
        match v {
            Ok(Some(v)) => {
                self.let_set(
                    k,
                    match v {
                        Data::List(v) => Data::List(v),
                        Data::Map(m) => Data::Map(m),
                        v => Data::List(vec![v]),
                    },
                );
                Ok(())
            }
            Ok(None) => Ok(()),
            Err(e) => Err(e),
        }
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

    pub fn for_each<F: FnMut(&str, &Data, usize)>(&self, mut f: F) {
        self.0.borrow().for_each(&mut f)
    }

    pub fn as_map(&self) -> BTreeMap<String, Data> {
        let mut mp = BTreeMap::new();
        self.for_each(|k, v, _| {
            mp.insert(k.to_string(), v.clone());
        });
        mp
    }
}

//consider where to put this func
pub fn alias(s: &str, store: &Store) -> Option<String> {
    let mut res = None;
    store.do_with("RU_ALIAS", |d| {
        if let Data::Map(m) = d {
            for (k, v) in m {
                if s.starts_with(k) {
                    let mut vs = v.to_string();
                    let push = &s[k.len()..];
                    match push.chars().next() {
                        Some(' ') | Some('\t') | None => {
                            vs.push_str(&s[k.len()..]);
                            res = Some(vs);
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }
    });
    res
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
