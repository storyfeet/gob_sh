//Manages carring all the messages to the user
use crate::cursor::Cursor;
use crate::guess_manager::*;
use crate::highlight::Highlight;
use crate::ui;
use crate::RT;
use bogobble::partial::ranger::Ranger;
use std::fmt::Write;
use std::io::Write as IWrite;
use std::path::PathBuf;
use termion::color;

#[derive(Debug, Clone)]
pub struct Prompt {
    pr_line: String,
    built: String,
    restore: Option<Cursor>,
    pub options: Option<(Ranger, Vec<String>)>,
    pub message: Option<String>,
    pub cursor: Cursor,
    pub guess_man: GuessManager,
    pub highlight: Highlight,
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
            guess_man: GuessManager::new(Some(20)),
            highlight: Highlight::empty(),
        }
    }

    pub fn set_highlight(&mut self, s: &str) {
        match Highlight::from_str(s) {
            Ok(v) => self.highlight = v,
            Err(e) => self.message = Some(format!("highlight parse error: {}", e)),
        }
    }

    pub fn reset(&mut self, pr_line: String, rt: &mut RT) {
        self.pr_line = pr_line;
        self.options = None;
        self.message = None;
        self.restore = None;
        self.built = String::new();
        self.cursor = Cursor::at_end(String::new());
        self.guess_man.clear();
        self.print(rt);
    }

    pub fn esc(&mut self, rt: &mut RT) {
        self.unprint(rt);
        self.clear_help();
        self.restore = None;
        self.guess_man.clear();
        self.print(rt);
    }

    pub fn set_guesses(&mut self, v: Vec<String>) {
        let go_up = v.len() > 0;
        self.guess_man.set_guesses(v);
        if go_up {
            self.up();
        }
    }

    pub fn up(&mut self) {
        match self.guess_man.next() {
            Some(s) => self.replace_cursor(s),
            None => {}
        }
    }

    pub fn down(&mut self) {
        match self.guess_man.prev() {
            Some(s) => self.replace_cursor(s),
            None => self.do_restore(),
        }
    }

    pub fn do_restore(&mut self) {
        match self.restore.take() {
            Some(mut d) => std::mem::swap(&mut d, &mut self.cursor),
            None => {}
        }
    }

    pub fn replace_cursor(&mut self, s: String) {
        let mut new_cursor = Cursor::at_end(s);
        std::mem::swap(&mut self.cursor, &mut new_cursor);
        if let None = &self.restore {
            self.restore = Some(new_cursor);
        }
    }

    /*
    pub fn replace_line(&mut self, line: Option<&String>) {
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
    }
    */

    pub fn clear_help(&mut self) {
        self.options = None;
        self.message = None;
    }

    pub fn print(&mut self, rt: &mut RT) {
        let pass1 = self.build(&self.cursor.s, true);
        self.built = self.build(self.cursor.on_s(), false);
        ui::print(&pass1);
        ui::unprint(&pass1, rt, false);
        ui::print(&self.built);
        rt.flush().ok();
    }

    pub fn print_end(&mut self, rt: &mut RT) {
        self.built = self.build(&self.cursor.s, false);
        ui::print(&self.built);
        rt.flush().ok();
    }

    pub fn unprint(&self, rt: &mut RT) {
        ui::unprint(&self.built, rt, true);
    }

    pub fn build(&self, line: &str, with_ops: bool) -> String {
        let mut res = String::new();
        let (pwidth, _) = termion::terminal_size().unwrap_or((50, 50));

        //println!("origin = {:?}\r\n", line);

        let line = build_line(line, &self.highlight);

        //println!("result = {:?}\r\n\n\n\n\n", line);

        if let Some(m) = &self.message {
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
    pub fn add_char(&mut self, c: char) {
        if let Some((pos, mut ops)) = self.options.take() {
            if let Some(n) = ui::char_as_int(c) {
                if ops.len() <= 10 {
                    match ops.get(n) {
                        Some(v) => {
                            //TODO -- make so cursor moves with changes
                            self.cursor.replace_range(pos, v);
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
                return;
            }
        }
        self.cursor.add_char(c);
        self.clear_help();
    }

    pub fn del_char(&mut self) {
        self.clear_help();
        self.cursor.del_char();
    }

    pub fn do_print<F: FnOnce(&mut Prompt) -> T, T>(&mut self, rt: &mut RT, f: F) -> T {
        self.unprint(rt);
        let r = f(self);
        self.print(rt);
        r
    }

    pub fn do_cursor<F: Fn(&mut Cursor) -> T, T>(&mut self, rt: &mut RT, f: F) -> T {
        self.unprint(rt);
        let r = f(&mut self.cursor);
        self.print(rt);
        r
    }
}

pub fn build_line<'a>(l: &str, hl: &Highlight) -> String {
    match hl.highlight(l) {
        Ok(s) => {
            //let s = bogobble::partial::mark_list::mark_str(&v, l).expect("Marking out of String");
            //       println!("parsed line = '{}'\r\n\n", s);
            let res = format!("{}{}", s, color::Fg(color::Reset));
            let res = res.replace("\n", "\n... ");
            res
        }
        Err(e) => {
            //      println!("Parse Error '{}',\r\n\n\n", e);
            match e.index {
                Some(0) | None => format!(
                    "{}{}{}",
                    color::Fg(color::LightRed),
                    l,
                    color::Fg(color::Reset),
                )
                .replace("\n", "\n... "),
                Some(n) => format!(
                    "{}{}{}{}",
                    build_line(&l[0..n], hl),
                    color::Fg(color::LightRed),
                    &l[n..].replace("\n", "\n... "),
                    color::Fg(color::Reset)
                ),
            }
        }
    }
}
