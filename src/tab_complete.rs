use crate::data::Data;
use crate::parser;
use crate::store::Store;
use crate::str_util;
use bogobble::traits::*;
use chrono::*;
use ru_history::HistoryStore;
use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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

pub fn tab_complete_args(s: &str, c: &str, store: &mut Store) -> anyhow::Result<Vec<String>> {
    // println!("Tab complete args \n\r\n");

    let mut op = None;
    store.do_with("RU_COMPLETE", |m| {
        if let Data::Map(mp) = m {
            op = mp.get(c).map(|c| c.clone())
        }
    });

    let mut res = Vec::new();
    if let Some(a_str) = op {
        let args = parser::ArgsP
            .parse_s(&a_str.to_string())
            .map_err(|e| e.strung())?;
        let a_done = args.run_s_vec(store, 2)?;

        for a in &a_done {
            if a.starts_with(s) {
                res.push(a.to_string());
            }
        }
    }
    Ok(res)
}

pub fn tab_complete_prog(s: &str) -> Vec<String> {
    let list: Vec<String> = prog_matches(s).into_iter().collect();
    list
}

pub fn prog_matches(s: &str) -> BTreeSet<String> {
    let mut res = BTreeSet::new();
    let v = std::env::var("PATH").unwrap_or("".to_string());
    for a in v.split(":") {
        folder_prog_matches(a, s, &mut res);
    }
    res
}

fn folder_prog_matches<P: AsRef<Path>>(folder: P, s: &str, v: &mut BTreeSet<String>) {
    let entries = match std::fs::read_dir(folder.as_ref()) {
        Ok(dirs) => dirs,
        _ => return,
    };
    for e in entries {
        match e {
            Ok(entry) => {
                let ft = match entry.file_type() {
                    Ok(f) => f,
                    _ => continue,
                };
                if ft.is_dir() {
                    //                folder_prog_matches(entry.path(), s, v);
                } else {
                    let fname = entry.file_name();
                    let fstr = fname.to_string_lossy();
                    if fstr.starts_with(s) {
                        v.insert(fstr.to_string());
                    }
                }
            }
            _ => continue, // If an issue, carry on
        }
    }
}

pub fn tab_complete_path(src: &str) -> Vec<String> {
    let (s, hd) = match src.starts_with("~") {
        true => {
            let hd = std::env::var("HOME").unwrap_or("".to_string());
            (src.replacen("~", &hd, 1), Some(hd))
        }
        false => (src.to_string(), None),
    };
    let sg = format!("{}{}", s.replace("\\ ", " ").trim_end_matches("*"), "*");
    let g = glob::glob(&sg)
        .map(|m| {
            m.filter_map(|a| a.ok())
                .map(|d| dir_slash(&d, hd.as_ref()))
                .collect()
        })
        .unwrap_or(Vec::new());
    if g.len() > 0 {
        if let Some(s) = all_strs_agree(g.iter(), s.len()) {
            return vec![s];
        }
    }
    g
    /*match g.len() {
        0 => return v,
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
    }*/
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

pub fn save_history(hs: &mut HistoryStore) -> anyhow::Result<()> {
    let (y, m) = year_month(SystemTime::now());
    let hpath = history_path();
    std::fs::create_dir_all(hpath).ok();
    let path = on_year_month(&history_path(), y, m);
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .write(true)
        .open(path)?;
    hs.write_to(&mut f, false)?;
    Ok(())
}
