//Manages carring all the messages to the user
use crate::cursor::Cursor;
use crate::ui;
use crate::RT;
use bogobble::traits::*;
use std::fmt::Write;
use std::io::Write as IWrite;
use std::path::PathBuf;
use termion::color;

#[derive(Debug, Clone)]
pub struct Prompt {
    pr_line: String,
    built: String,
    restore: Option<Cursor>,
    pub options: Option<(usize, Vec<String>)>,
    pub message: Option<String>,
    pub cursor: Cursor,
}

impl Prompt {
    pub fn new(pr_line: String) -> Self {
        Prompt {
            pr_line,
            options: None,
            message: None,
            restore: None,
            built: String::new(),
            cursor: Cursor::at_end(String::new()),
        }
    }

    pub fn esc(&mut self, rt: &mut RT) {
        self.unprint(rt);
        self.clear_help();
        self.restore = None;
        self.print(rt);
    }

    pub fn replace_line(&mut self, line: Option<&String>, rt: &mut RT) {
        self.unprint(rt);
        self.clear_help();
        let new_cursor = line.map(|l| Cursor::at_end(l.clone()));
        match (&mut self.restore, new_cursor) {
            (Some(_), Some(nc)) => self.cursor = nc,
            (None, Some(mut nc)) => {
                std::mem::swap(&mut nc, &mut self.cursor);
                self.restore = Some(nc);
            }
            (Some(ref mut v), None) => {
                std::mem::swap(&mut self.cursor, v);
                self.restore = None;
            }
            _ => {} //self.line = "".to_string(),
        }
        self.print(rt);
    }

    pub fn clear_help(&mut self) {
        self.options = None;
        self.message = None;
    }

    pub fn reset(&mut self, pr_line: String, rt: &mut RT) {
        *self = Self::new(pr_line);
        self.print(rt);
    }

    pub fn print(&mut self, rt: &mut RT) {
        let pass1 = self.build(&self.cursor.s, true);
        ui::print(&pass1);

        ui::unprint(&pass1, rt, false);
        self.built = self.build(self.cursor.on_s(), false);
        ui::print(&self.built);
        rt.flush().ok();
    }

    pub fn unprint(&self, rt: &mut RT) {
        ui::unprint(&self.built, rt, true);
    }

    pub fn build(&self, line: &str, with_ops: bool) -> String {
        let mut res = String::new();
        let (pwidth, _) = termion::terminal_size().unwrap_or((50, 50));

        let (line, err) = build_line(line);

        let mess = match err {
            Some(e) => Some(e.to_string()),
            None => self.message.clone(),
        };

        if let Some(m) = &mess {
            write!(res, "[{}]\n\r", m).ok();
        }
        res.push_str(&self.pr_line);
        write!(res, "{}", line).ok();
        if let (Some((_, ops)), true) = (&self.options, with_ops) {
            match ops.len() {
                n if n <= 10 => {
                    for (n, o) in ops.iter().enumerate() {
                        write!(res, "\n{}:  {}", n, o).ok();
                    }
                }
                _ => {
                    for (n, o) in ops.iter().enumerate() {
                        let s = match PathBuf::from(o).file_name() {
                            Some(s) => s.to_string_lossy().to_string(),
                            None => o.to_string(),
                        };
                        let nl = match n % (pwidth as usize / 25) {
                            0 => "\n",
                            _ => "",
                        };
                        write!(res, "{}{:0>2}:  {:20}", nl, n, s,).ok();
                    }
                }
            }
        }
        res
    }
    pub fn add_char(&mut self, c: char, rt: &mut RT) {
        self.unprint(rt);
        if let Some((pos, mut ops)) = self.options.take() {
            if let Some(n) = ui::char_as_int(c) {
                if ops.len() <= 10 {
                    match ops.get(n) {
                        Some(v) => {
                            //TODO -- make so cursor moves with changes
                            self.cursor.s.replace_range(pos.., v);
                            self.clear_help();
                        }
                        None => {
                            self.message = Some("Selection Not Valid".to_string());
                            self.options = Some((pos, ops));
                        }
                    }
                } else {
                    //More than 10 options
                    ops = ops.into_iter().skip(n * 10).take(10).collect();
                    self.options = Some((pos, ops))
                }
                self.print(rt);
                return;
            }
        }
        self.cursor.add_char(c);
        self.clear_help();

        self.print(rt);
    }

    pub fn del_char(&mut self, rt: &mut RT) {
        self.unprint(rt);
        self.clear_help();
        self.cursor.del_char();
        self.print(rt);
    }

    pub fn del_line(&mut self, rt: &mut RT) {
        self.unprint(rt);
        self.clear_help();
        // TODOui::del_line(&mut self.line);
        self.cursor.del_char();
        self.print(rt);
    }

    pub fn left(&mut self, rt: &mut RT) -> bool {
        self.unprint(rt);
        let r = self.cursor.left();
        self.print(rt);
        r
    }
}

pub fn build_line<'a>(l: &str) -> (String, Option<String>) {
    match crate::partial::Lines.parse_s(l) {
        Ok(v) => {
            let s = bogobble::partial::mark_list::mark_str(&v, l).expect("Marking out of String");
            let res = format!("{}{}", s, color::Fg(color::Reset));
            let res = res.replace("\n", "\n... ");
            (res, None)
        }
        Err(e) => {
            let res = format!(
                "{}{}{}",
                color::Fg(color::LightRed),
                l,
                color::Fg(color::Reset),
            );
            let res = res.replace("\n", "\n... ");
            (res, Some(e.to_string()))
        }
    }
}
