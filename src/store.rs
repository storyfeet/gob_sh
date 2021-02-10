use std::collections::BTreeMap;
use std::fmt::{self, Display};

use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct AStore {
    ch: mpsc::Sender<Job>,
}

impl AStore {
    pub async fn new() -> Self {
        let (ch, ch_r) = mpsc::channel(5);
        tokio::spawn(scope_handler(ch_r));
        Self { ch }
    }

    pub async fn get(&self, s: String) -> Option<Data> {
        let (ret_s, ret_r) = oneshot::channel();
        self.ch.send(Job::Get(s, ret_s)).await.ok()?;
        ret_r.await.unwrap_or(None)
    }

    pub async fn let_set(&self, s: String, d: Data) {
        self.ch.send(Job::Let(s, d)).await.ok();
    }

    pub async fn set(&self, s: String, d: Data) {
        self.ch.send(Job::Set(s, d)).await.ok();
    }
}

pub enum Job {
    Get(String, oneshot::Sender<Option<Data>>),
    Set(String, Data),
    Let(String, Data),
}

///Consider accepting an optional parent to this method.
pub async fn scope_handler(mut ch_r: mpsc::Receiver<Job>) {
    let mut store = Store::new();
    while let Some(j) = ch_r.recv().await {
        match j {
            Job::Let(s, d) => store.let_set(s, d),
            Job::Set(s, d) => store.set(s, d),
            Job::Get(s, ch_s) => {
                drop(ch_s.send(store.get(&s)));
            }
        }
    }
}

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
}
