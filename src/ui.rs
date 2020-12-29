use crate::RT;
use bogobble::traits::*;
use std::io::Write;
use termion::*;

pub fn line_count(s: &str, w: usize) -> usize {
    let mut res = 0;
    let mut cw = 0;
    for c in s.chars() {
        match c {
            '\n' => {
                res += 1;
                cw = 0;
            }
            _ => {
                if cw + 3 > w {
                    res += 1;
                    cw = 0;
                } else {
                    cw += 1;
                }
            }
        }
    }
    res
}

pub fn del_char(s: &mut String) {
    let l = s.len();
    for x in 1..6 {
        if l < x {
            return;
        }
        if let Some(_) = s.as_str().get(l - x..) {
            s.remove(l - x);
            return;
        }
    }
}

pub fn del_line(s: &mut String) {
    let l = s.len();
    for x in 1..l {
        if let Some('\n') = s.as_str().get(l - x..).and_then(|s| s.chars().next()) {
            s.replace_range(l - x.., "");
            return;
        }
    }
    s.clear();
}

pub fn unprint(s: &str, _rt: &mut RT) {
    let (t_width, _) = termion::terminal_size().unwrap_or((50, 50));
    let lcount = crate::ui::line_count(s, t_width as usize);
    if lcount > 0 {
        print!("{}", cursor::Up(lcount as u16));
    }
    print!("\r{}", clear::AfterCursor,);
}

pub fn print(s: &str, rt: &mut RT) {
    //let lines: Vec<&str> = self.line.split("\n").collect();
    //parse first
    let s2 = match crate::partial::Statement.parse_s(s) {
        Ok(v) => bogobble::partial::mark_list::mark_str(&v, s).expect("Marking out of String"),
        Err(e) => format!(
            "{}{}{}{}",
            color::Fg(color::LightRed),
            s,
            color::Fg(color::Reset),
            e,
        ),
    };
    assert!(s2.len() >= s.len(), "Marked string should be longer");
    let mut pre = "> ";
    for a in s2.split("\n") {
        print!("{}{}", pre, a);
        pre = "\n\r... ";
    }
    rt.flush().ok();
}
