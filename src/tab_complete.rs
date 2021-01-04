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
    pub pos: Option<usize>,
}

impl HistoryStore {
    pub fn new() -> Self {
        Self {
            mp: BTreeMap::new(),
            recent: Vec::new(),
            pos: None,
        }
    }

    pub fn push_command(&mut self, cmd: String) {
        let time = SystemTime::now();
        let pwd = std::env::current_dir().unwrap_or(PathBuf::from(""));
        if let Some(mut cv) = self.mp.get_mut(&cmd) {
            if !cv.pwds.contains(&pwd) {
                cv.pwds.push(pwd.clone());
            }
            cv.time = time;
            cv.hits += 1;
            return;
        }
        self.recent.push(cmd.clone());
        if self.recent.len() > 200 {
            self.recent.remove(0);
        }

        self.mp.insert(
            cmd,
            HistoryItem {
                pwds: vec![pwd],
                time,
                hits: 1,
            },
        );
    }

    pub fn cmd_complete<'a>(&'a mut self, cmd: &str) -> Vec<(&'a String, &'a HistoryItem)> {
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

    fn select_recent(&mut self, n: usize) -> Option<&String> {
        let l = self.recent.len();
        if n >= l {
            self.pos = None;
            return None;
        }
        self.pos = Some(n);
        self.recent.get(l - 1 - n)
    }
}

#[derive(Clone, Debug)]
pub struct HistoryItem {
    pwds: Vec<PathBuf>,
    time: SystemTime,
    hits: usize,
}
