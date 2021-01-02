use std::path::Path;

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
