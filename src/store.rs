use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::path::Path;

use crate::parser;
use bogobble::traits::*;
use tokio::io::AsyncReadExt;
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct AStore {
    ch: mpsc::Sender<Job>,
    global: mpsc::Sender<Job>,
}

impl AStore {
    pub async fn new_global() -> Self {
        let (global, global_r) = mpsc::channel(5);

        tokio::spawn(global_handler(global_r));
        Self {
            ch: global.clone(),
            global,
        }
    }

    pub async fn get(&self, s: String) -> Option<Data> {
        get(&self.ch, s).await
    }

    pub async fn let_set(&self, s: String, d: Data) {
        self.ch.send(Job::Let(s, d)).await.ok();
    }

    pub async fn set(&self, s: String, d: Data) {
        set(&self.ch, s, d).await
    }

    pub async fn child(&self) -> AStore {
        let (ch, ch_r) = mpsc::channel(5);
        tokio::spawn(child_handler(ch_r, self.ch.clone()));
        Self {
            ch,
            global: self.global.clone(),
        }
    }

    pub async fn source_path<P: AsRef<Path>>(self, p: P) -> anyhow::Result<String> {
        let mut f = match tokio::fs::File::open(p).await {
            Ok(f) => f,
            Err(_) => return Ok(self.do_prompt().await), //No file is not an error
        };
        let mut buf = String::new();
        f.read_to_string(&mut buf).await?;
        let p = parser::Lines.parse_s(&buf).map_err(|e| e.strung())?;
        for v in p {
            v.run(&self).await.ok();
        }

        //do prompt

        Ok(self.do_prompt().await)
    }

    pub async fn do_prompt(&self) -> String {
        let s = self
            .get("RU_PROMPT".to_string())
            .await
            .map(|s| s.to_string())
            .unwrap_or(">>".to_string());
        let p = match parser::QuotedString.parse_s(&s) {
            Ok(v) => v,
            Err(e) => return format!("__{}__>>", e),
        };
        match p.run(self.clone()).await {
            Ok(s) => s.to_string(),
            Err(e) => format!("__{}__>>", e),
        }
    }
}

async fn set(ch: &mpsc::Sender<Job>, s: String, d: Data) {
    let (ch_s, ch_r) = oneshot::channel();
    ch.send(Job::Set(s, d, ch_s)).await.ok();
    drop(ch_r.await)
}

pub async fn get(ch: &mpsc::Sender<Job>, s: String) -> Option<Data> {
    let (ret_s, ret_r) = oneshot::channel();
    ch.send(Job::Get(s, ret_s)).await.ok()?;
    ret_r.await.unwrap_or(None)
}

pub enum Job {
    Get(String, oneshot::Sender<Option<Data>>),
    Set(String, Data, oneshot::Sender<()>),
    Let(String, Data),
}

pub async fn global_handler(mut ch_r: mpsc::Receiver<Job>) {
    let mut store = BTreeMap::new();
    while let Some(j) = ch_r.recv().await {
        match j {
            Job::Let(s, d) => drop(store.insert(s, d)),
            Job::Set(s, d, ch) => {
                store.insert(s, d);
                drop(ch.send(()))
            }
            Job::Get(s, ch_s) => {
                drop(ch_s.send(store.get(&s).map(|c| c.clone())));
            }
        }
    }
}

pub async fn child_handler(mut ch_r: mpsc::Receiver<Job>, parent: mpsc::Sender<Job>) {
    let mut store = BTreeMap::new();
    while let Some(j) = ch_r.recv().await {
        match j {
            Job::Let(s, d) => drop(store.insert(s, d)),
            Job::Set(s, d, ch) => match store.get(&s) {
                Some(_) => drop(store.insert(s, d)),
                None => {
                    set(&parent, s, d).await;
                    drop(ch.send(()));
                }
            },
            Job::Get(s, ch_s) => {
                let res = match store.get(&s) {
                    None => get(&parent, s).await,
                    Some(s) => Some(s.clone()),
                };
                drop(ch_s.send(res));
            }
        }
    }
}

/*
#[derive(Debug, Clone)]
struct Store {
    scopes: Vec<BTreeMap<String, Data>>,
}
*/

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
