//Manages carring all the messages to the user
use crate::RT;
use bogobble::traits::*;
use std::fmt::Write;
use std::io::Write as IWrite;
use termion::color;

#[derive(Debug, Clone)]
pub struct Prompt {
    pr_line: String,
    built: String,
    options: Option<(usize, Vec<String>)>,
    message: Option<String>,
    pub line: String,
}

impl Prompt {
    pub fn new() -> Self {
        Prompt {
            pr_line: String::new(),
            options: None,
            message: None,
            line: String::new(),
            built: String::new(),
        }
    }

    pub fn reset(&mut self, pr_line: String, rt: &mut RT) {
        *self = Self {
            pr_line,
            ..Self::new()
        };
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

    pub fn build_line(&self) -> String {
        let mut res = String::new();
        match crate::partial::Lines.parse_s(&self.line) {
            Ok(v) => {
                let s = bogobble::partial::mark_list::mark_str(&v, &self.line)
                    .expect("Marking out of String");
                write!(res, "{}{}", s, color::Fg(color::Reset)).ok()
            }
            Err(e) => write!(
                res,
                "{}{}{}{}",
                color::Fg(color::LightRed),
                &self.line,
                color::Fg(color::Reset),
                e,
            )
            .ok(),
        };
        res.replace("\n", "\n... ")
    }
    pub fn build(&mut self) {
        self.built.clear();
        self.built.push_str(&self.pr_line);

        if let Some(m) = &self.message {
            write!(self.built, "[{}] >", m).ok();
        }
        let line = self.build_line();
        //println!("Line === {}", line);
        write!(self.built, "{}", line).ok();
        if let Some((_, ops)) = &self.options {
            for (n, o) in ops.iter().enumerate() {
                write!(self.built, "\n{}:  {}", n, o).ok();
            }
        }
    }
    pub fn add_char(&mut self, c: char, rt: &mut RT) {
        self.unprint(rt);
        self.line.push(c);
        self.print(rt);
    }

    pub fn del_char(&mut self, rt: &mut RT) {
        self.unprint(rt);
        crate::ui::del_char(&mut self.line);
        self.print(rt);
    }

    pub fn del_line(&mut self, rt: &mut RT) {
        self.unprint(rt);
        crate::ui::del_line(&mut self.line);
        self.print(rt);
    }
}
