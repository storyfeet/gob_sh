use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ops::Bound;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub enum Complete {
    One(String),
    Many(Vec<String>),
    None,
}

fn dir_slash(p: &Path) -> String {
    match p.metadata().map(|dt| dt.is_dir()) {
        Ok(true) => format!("{}/", p.display()),
        _ => p.display().to_string(),
    }
}

pub fn tab_complete_path(s: &str) -> Complete {
    let sg = format!("{}{}", s.trim_end_matches("*"), "*");
    let g = glob::glob(&sg)
        .map(|m| m.filter_map(|a| a.ok()).collect())
        .unwrap_or(Vec::new());
    match g.len() {
        0 => return Complete::None,
        1 => {
            let tg = &g[0];
            Complete::One(dir_slash(tg))
        }
        _ => Complete::Many(g.into_iter().map(|d| dir_slash(&d)).collect()),
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

    pub fn push_command(&mut self, cmd: String) {
        let time = SystemTime::now();
        let pwd = std::env::current_dir().unwrap_or(PathBuf::from(""));
        match self.mp.get_mut(&cmd) {
            Some(mut cv) => {
                if !cv.pwds.contains(&pwd) {
                    cv.pwds.push(pwd.clone());
                }
                cv.time = time;
                cv.hits += 1;
            }
            None => {
                self.mp.insert(
                    cmd.clone(),
                    HistoryItem {
                        pwds: vec![pwd],
                        time,
                        hits: 1,
                    },
                );
            }
        }

        self.recent.push(cmd);
        if self.recent.len() > 200 {
            self.recent.remove(0);
        }
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

#[derive(Clone, Debug)]
pub struct HistoryItem {
    pwds: Vec<PathBuf>,
    time: SystemTime,
    hits: usize,
}
