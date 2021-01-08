use chrono::*;
use serde_derive::*;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::Write;
use std::ops::Bound;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub enum Complete {
    One(String),
    Many(Vec<String>),
    None,
}

fn dir_slash(p: &Path, td: Option<&String>) -> String {
    let mut s = p
        .to_str()
        .map(|s| match td {
            Some(hs) => format!("~{}", s.strip_prefix(hs).unwrap_or(s)),
            None => s.to_string(),
        })
        .map(|s| s.replace(" ", "\\ "))
        .unwrap_or(p.display().to_string());
    if let Ok(true) = p.metadata().map(|dt| dt.is_dir()) {
        s.push('/');
    }
    s
}

pub fn tab_complete_path(s: &str) -> Complete {
    let (s, hd) = match s.starts_with("~") {
        true => {
            let hd = std::env::var("HOME").unwrap_or("".to_string());
            (s.replacen("~", &hd, 1), Some(hd))
        }
        false => (s.to_string(), None),
    };
    let sg = format!("{}{}", s.replace("\\ ", " ").trim_end_matches("*"), "*");
    let g = glob::glob(&sg)
        .map(|m| m.filter_map(|a| a.ok()).collect())
        .unwrap_or(Vec::new());
    match g.len() {
        0 => return Complete::None,
        1 => {
            let tg = &g[0];
            Complete::One(dir_slash(tg, hd.as_ref()))
        }
        _ => Complete::Many(g.into_iter().map(|d| dir_slash(&d, hd.as_ref())).collect()),
    }
}

#[derive(Clone, Debug)]
pub struct HistoryStore {
    mp: BTreeMap<String, HistoryItem>,
    recent: Vec<String>,
    pub guesses: Option<Vec<String>>,
    pub pos: Option<usize>,
}

impl HistoryStore {
    pub fn new() -> Self {
        Self {
            mp: BTreeMap::new(),
            recent: Vec::new(),
            pos: None,
            guesses: None,
        }
    }

    pub fn push_command(&mut self, cmd: String) -> anyhow::Result<()> {
        let time = SystemTime::now();
        let pwd = std::env::current_dir().unwrap_or(PathBuf::from(""));
        match self.mp.get_mut(&cmd) {
            Some(mut cv) => {
                if !cv.pwds.contains(&pwd) {
                    cv.pwds.push(pwd.clone());
                }
                cv.time = time;
                cv.hits += 1;
                HistorySaver::new(&cmd, &cv).save()?;
            }
            None => {
                let item = HistoryItem {
                    pwds: vec![pwd],
                    time,
                    hits: 1,
                };
                HistorySaver::new(&cmd, &item).save()?;
                self.mp.insert(cmd.clone(), item);
            }
        }

        self.recent.push(cmd);
        if self.recent.len() > 200 {
            self.recent.remove(0);
        }
        Ok(())
    }

    pub fn guess(&mut self, cmd: &str) -> bool {
        let mut g = self.cmd_complete(cmd);
        if g.len() == 0 {
            return false;
        }
        let cpwd = std::env::current_dir().unwrap_or(PathBuf::from(""));
        g.sort_by(|(_, a), (_, b)| {
            let mut sca = a.hits;
            if a.pwds.contains(&cpwd) {
                sca += 10;
            }
            let mut scb = b.hits;
            if b.pwds.contains(&cpwd) {
                scb += 10;
            }
            match a.time.cmp(&b.time) {
                Ordering::Greater => sca += 10,
                Ordering::Less => scb += 10,
                _ => {}
            }
            sca.cmp(&scb)
        });
        self.guesses = Some(g.into_iter().map(|(a, _)| a.clone()).collect());
        true
    }

    pub fn cmd_complete<'a>(&'a mut self, cmd: &str) -> Vec<(&'a String, &'a HistoryItem)> {
        if cmd == "" {
            return self.mp.iter().collect();
        }
        //Get exclude str with next char.
        let mut cend = cmd.to_string();
        let ec = crate::ui::del_char(&mut cend).and_then(|c| std::char::from_u32((c as u32) + 1));
        match ec {
            Some(c) => cend.push(c),
            None => return Vec::new(),
        }
        self.mp
            .range::<str, _>((Bound::Included(cmd), Bound::Excluded(cend.as_str())))
            .collect()
    }

    pub fn up_recent(&mut self) -> Option<&String> {
        match self.pos {
            Some(n) => self.select_recent(n + 1),
            None => self.select_recent(0),
        }
    }

    pub fn down_recent(&mut self) -> Option<&String> {
        match self.pos {
            Some(0) => {
                self.pos = None;
                None
            }
            Some(n) => self.select_recent(n - 1),
            None => self.select_recent(0),
        }
    }

    pub fn select_recent(&mut self, n: usize) -> Option<&String> {
        let v = match &self.guesses {
            Some(g) => g,
            None => &self.recent,
        };

        let l = v.len();
        if n >= l {
            self.pos = None;
            return None;
        }
        self.pos = Some(n);
        v.get(l - 1 - n)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryItem {
    pwds: Vec<PathBuf>,
    time: SystemTime,
    hits: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct SaveArray<'a> {
    item: Vec<&'a HistorySaver>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistorySaver {
    cmd: String,
    pwds: Vec<PathBuf>,
    time: SystemTime,
    hits: usize,
}

impl HistorySaver {
    pub fn new(cmd: &str, item: &HistoryItem) -> Self {
        HistorySaver {
            cmd: cmd.to_string(),
            pwds: item.pwds.clone(),
            time: item.time,
            hits: item.hits,
        }
    }

    //Currently just append to file and hope for the best.
    pub fn save(&self) -> anyhow::Result<()> {
        let a = SaveArray { item: vec![self] };
        let mut tdir = PathBuf::from(std::env::var("HOME")?);
        tdir.push(".config/rushell/history");
        let ch_t: DateTime<offset::Local> = DateTime::from(self.time);

        std::fs::create_dir_all(&tdir)?;
        let dt_s = ch_t.format("history_%Y_%m.toml").to_string();
        tdir.push(&dt_s);

        let tv = toml::Value::try_from(&a)?;
        let s = toml::to_string(&tv)?;

        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(&tdir)?;
        write!(f, "{}", &s)?;

        Ok(())
    }
}
