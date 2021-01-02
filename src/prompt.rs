//Manages carring all the messages to the user
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
    pub options: Option<(usize, Vec<String>)>,
    pub message: Option<String>,
    pub line: String,
}

impl Prompt {
    pub fn new(pr_line: String) -> Self {
        Prompt {
            pr_line,
            options: None,
            message: None,
            line: String::new(),
            built: String::new(),
        }
    }

    pub fn esc(&mut self, rt: &mut RT) {
        self.unprint(rt);
        self.clear_help();
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
        self.build();
        let mut pre = "";
        for l in self.built.split("\n") {
            print!("{}{}", pre, l);
            pre = "\n\r";
        }
        rt.flush().ok();
    }

    pub fn unprint(&self, rt: &mut RT) {
        let stp = console::strip_ansi_codes(&self.built);
        crate::ui::unprint(&stp, rt);
    }

    pub fn build_line<'a>(&'a self) -> (String, Option<String>) {
        match crate::partial::Lines.parse_s(&self.line) {
            Ok(v) => {
                let s = bogobble::partial::mark_list::mark_str(&v, &self.line)
                    .expect("Marking out of String");
                let res = format!("{}{}", s, color::Fg(color::Reset));
                let res = res.replace("\n", "\n... ");
                (res, None)
            }
            Err(e) => {
                let res = format!(
                    "{}{}{}",
                    color::Fg(color::LightRed),
                    &self.line,
                    color::Fg(color::Reset),
                );
                let res = res.replace("\n", "\n... ");
                (res, Some(e.to_string()))
            }
        }
    }
    pub fn build(&mut self) {
        self.built.clear();

        let (line, err) = self.build_line();
        if let Some(e) = err {
            self.message = Some(e);
        }

        if let Some(m) = &self.message {
            write!(self.built, "[{}]\n\r", m).ok();
        }
        self.built.push_str(&self.pr_line);
        write!(self.built, "{}", line).ok();
        if let Some((_, ops)) = &self.options {
            match ops.len() {
                n if n <= 10 => {
                    for (n, o) in ops.iter().enumerate() {
                        write!(self.built, "\n{}:  {}", n, o).ok();
                    }
                }
                _ => {
                    for (n, o) in ops.iter().enumerate() {
                        let s = match PathBuf::from(o).file_name() {
                            Some(s) => s.to_string_lossy().to_string(),
                            None => o.to_string(),
                        };
                        let nl = match n % 3 {
                            0 => "\n",
                            _ => "",
                        };
                        write!(self.built, "{}{:0>2}:  {:20}", nl, n, s,).ok();
                    }
                }
            }
        }
    }
    pub fn add_char(&mut self, c: char, rt: &mut RT) {
        self.unprint(rt);
        if let Some((pos, mut ops)) = self.options.take() {
            if let Some(n) = crate::ui::char_as_int(c) {
                if ops.len() <= 10 {
                    match ops.get(n) {
                        Some(v) => {
                            self.line.replace_range(pos.., v);
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
        self.line.push(c);
        self.clear_help();

        self.print(rt);
    }

    pub fn del_char(&mut self, rt: &mut RT) {
        self.unprint(rt);
        self.clear_help();
        crate::ui::del_char(&mut self.line);
        self.print(rt);
    }

    pub fn del_line(&mut self, rt: &mut RT) {
        self.unprint(rt);
        self.clear_help();
        crate::ui::del_line(&mut self.line);
        self.print(rt);
    }
}
