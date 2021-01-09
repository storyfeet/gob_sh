use crate::RT;
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
                if cw >= w {
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

pub fn del_char(s: &mut String) -> Option<char> {
    let l = s.len();
    for x in 1..6 {
        if l < x {
            return None;
        }
        if let Some(_) = s.as_str().get(l - x..) {
            let c = s[(l - x)..].chars().next();
            s.remove(l - x);
            return c;
        }
    }
    None
}

///```rust
/// use gob_ion::ui::del_n;
/// assert_eq!(del_n("hello",3),"he");
///```
pub fn del_n(s: &str, n: usize) -> &str {
    let mut done = 0;
    let l = s.len();
    for x in 1..l {
        if let Some(_) = s.get(l - x..) {
            done += 1;
            if done >= n {
                return &s[..l - x];
            }
        }
    }
    ""
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

pub fn print(s: &str) {
    let mut pre = "";
    for l in s.split("\n") {
        print!("{}{}", pre, l);
        pre = "\n\r";
    }
}

pub fn unprint(s: &str, _rt: &mut RT, del: bool) {
    let s = console::strip_ansi_codes(s);
    let (t_width, _) = termion::terminal_size().unwrap_or((50, 50));
    let lcount = crate::ui::line_count(&s, t_width as usize);
    if lcount > 0 {
        print!("{}", cursor::Up(lcount as u16));
    }
    print!("\r");
    if del {
        print!("{}", clear::AfterCursor,);
    }
}

pub fn char_as_int(c: char) -> Option<usize> {
    match c {
        n if n >= '0' && n <= '9' => Some(n as usize - 48),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_del_n() {
        assert_eq!(del_n("hello", 5), "");
        assert_eq!(del_n("hello", 6), "");
        assert_eq!(del_n("hello", 3), "he");
        assert_eq!(del_n("我不是中国人", 2), "我不是中");
    }
}
