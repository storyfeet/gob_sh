use crate::str_util;
use chrono::*;
use ru_history::HistoryStore;
use std::io::ErrorKind;
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

pub fn all_strs_agree<I: Iterator<Item = S>, S: AsRef<str>>(
    mut it: I,
    min_len: usize,
) -> Option<String> {
    let res = it.next()?.as_ref().to_string();
    let mut max = res.len();
    for v in it {
        max = str_util::str_agree(&res[..max], v.as_ref());
        if max <= min_len {
            return None;
        }
    }
    Some(res[..max].to_string())
}

pub fn tab_complete_path(src: &str) -> Complete {
    let (s, hd) = match src.starts_with("~") {
        true => {
            let hd = std::env::var("HOME").unwrap_or("".to_string());
            (src.replacen("~", &hd, 1), Some(hd))
        }
        false => (src.to_string(), None),
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
        _ => {
            let v: Vec<String> = g.into_iter().map(|d| dir_slash(&d, hd.as_ref())).collect();
            match all_strs_agree(v.iter(), s.len()) {
                Some(s) => return Complete::One(s),
                None => Complete::Many(v),
            }
        }
    }
}

fn date_to_history_path(t: SystemTime) -> PathBuf {
    let res = history_path();
    let (y, m) = year_month(t);
    on_year_month(&res, y, m)
}

fn history_path() -> PathBuf {
    let mut tdir = PathBuf::from(std::env::var("HOME").unwrap_or(String::new()));
    tdir.push(".config/rushell/history");
    tdir
}

fn on_year_month(p: &Path, y: i32, m: u32) -> PathBuf {
    let dt_s = format!("history_{}_{}.fd", y, m);
    p.join(&dt_s)
}

fn year_month(t: SystemTime) -> (i32, u32) {
    let dt: DateTime<offset::Local> = DateTime::from(t);
    (dt.year(), dt.month())
}

//Currently just append to file and hope for the best.
pub fn load_history(months: u32, hist: &mut HistoryStore) -> anyhow::Result<()> {
    let (y, m) = year_month(SystemTime::now());
    let path = history_path();

    for n in 1..=months {
        let sub = months - n;
        let y2 = y - (sub as i32 / 12);
        let m2 = ((m + 11 - sub as u32) % 12) + 1;
        let p2 = on_year_month(&path, y2, m2);

        match std::fs::read_to_string(p2) {
            Ok(s) => ru_history::parse::parse_onto(hist, &s)?,
            Err(e) => match e.kind() {
                ErrorKind::NotFound => continue,
                _ => return Err(e.into()),
            },
        }
    }
    Ok(())
}
