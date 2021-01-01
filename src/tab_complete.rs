pub enum Complete {
    One(String),
    Many(Vec<String>),
    None,
}

pub fn tab_complete_path(s: &str) -> Complete {
    let sg = format!("{}{}", s, "*");
    let g = glob::glob(&sg)
        .map(|m| m.filter_map(|a| a.ok()).collect())
        .unwrap_or(Vec::new());
    match g.len() {
        0 => return Complete::None,
        1 => {
            let tg = &g[0];
            Complete::One(match tg.metadata().map(|dt| dt.is_dir()) {
                Ok(true) => format!("{}/", tg.display()),
                _ => tg.display().to_string(),
            })
        }
        _ => Complete::Many(g.into_iter().map(|v| v.display().to_string()).collect()),
    }
}
