//! Some options for statements to run, or persistent data
use crate::cursor::Cursor;
use crate::partial::Item;
use crate::Action;
use bogobble::traits::*;
use termion::event::Key;

use crate::store::Store;
use crate::tab_complete::*;
use crate::{parser, prompt::Prompt, RT};
use std::io::Read;
use std::io::Write;
use std::path::Path;

#[derive(Clone, Debug)]

pub struct Shell {
    pub prompt: Prompt,
    pub store: Store,
    pub history: HistoryStore,
}

impl Shell {
    /// Invariants : Settings must always have at least one layer in scope.
    pub fn new() -> Shell {
        let mut history = HistoryStore::new();
        history.load_history();
        Shell {
            prompt: Prompt::new(">>".to_string()),
            store: Store::new(),
            history,
        }
    }

    pub fn do_print<T, F: Fn(&mut Self) -> T>(&mut self, rt: &mut RT, f: F) -> T {
        self.prompt.unprint(rt);
        let r = f(self);
        self.prompt.print(rt);
        r
    }

    fn tab_complete(&mut self) -> anyhow::Result<()> {
        self.prompt.clear_help();
        let c_line = &self.prompt.cursor.on_to_space();
        let clen = c_line.len();
        let top = crate::partial::Lines
            .parse_s(c_line)
            .map_err(|e| e.strung())?;

        if let Some(a) = top.find_at_end(c_line, |&i| i == Item::Arg) {
            let s = a.on_str(c_line);

            match crate::tab_complete::tab_complete_path(s) {
                Complete::None => self.prompt.message = Some(format!("Could not complete '{}'", s)),
                Complete::One(tc) => {
                    self.prompt
                        .cursor
                        .replace_range(a.start.unwrap_or(0)..clen, &tc);
                }
                Complete::Many(v) => self.prompt.options = Some((a.range().with_end(clen), v)),
            }
        }

        Ok(())
    }
    pub fn on_enter(&mut self, rt: &mut RT) {
        let c_line = &self.prompt.cursor.s;
        self.history.pos = None;
        self.history.guesses = None;
        match parser::Lines.parse_s(c_line) {
            Ok(v) => {
                let hist_r = self.history.push_command(c_line.clone());

                rt.suspend_raw_mode().ok();
                print!("\n\r");
                rt.flush().ok();
                for s in v {
                    match s.run(&mut self.store) {
                        Ok(false) => print!("\n\rOK - fail\n\r"),
                        Err(e) => print!("\n\rErr - {}\n\r", e),
                        _ => {}
                    }
                }
                rt.activate_raw_mode().ok();
                self.reset(rt);
                self.prompt.unprint(rt);
                match hist_r {
                    Err(e) => self.prompt.message = Some(e.to_string()),
                    Ok(_) => {} // self.prompt.message = Some(s),
                }
                self.prompt.print(rt);
            }
            Err(_) => self.do_print(rt, |sh| sh.prompt.add_char('\n', rt)),
        }
    }

    pub fn reset(&mut self, rt: &mut RT) {
        let pt = self
            .store
            .get("RU_PROMPT")
            .map(|d| d.to_string())
            .unwrap_or(String::from(">>"));
        let pt = match parser::QuotedString.parse_s(&pt) {
            Ok(v) => v
                .run(&mut self.store)
                .map(|s| s.to_string())
                .unwrap_or("PromptErr:>>".to_string()),
            Err(_) => pt,
        };
        self.prompt.reset(pt, rt);
        rt.flush().ok();
    }

    pub fn source_path<P: AsRef<Path>>(&mut self, p: P) -> anyhow::Result<()> {
        let mut f = std::fs::File::open(p)?;
        let mut buf = String::new();
        f.read_to_string(&mut buf)?;
        let p = parser::Lines.parse_s(&buf).map_err(|e| e.strung())?;
        for v in p {
            v.run(&mut self.store).ok();
        }
        Ok(())
    }

    pub fn do_key(&mut self, k: Key, rt: &mut RT) -> anyhow::Result<Action> {
        match k {
            Key::Ctrl('d') => return Ok(Action::Quit),
            Key::Char('\n') => self.on_enter(rt),
            Key::Char('\t') => {
                self.do_print(rt, Shell::tab_complete)
                    .expect("Could not complete tabs");
            }

            Key::Char(c) => self.prompt.add_char(c, rt),
            Key::Backspace => self.prompt.do_cursor(rt, Cursor::backspace),
            Key::Delete => self.prompt.do_cursor(rt, Cursor::del_char),
            Key::Ctrl('h') => self.prompt.do_cursor(rt, Cursor::del_line),
            Key::Esc => {
                self.prompt.esc(rt);
                self.history.guesses = None;
            }
            Key::Up => match self.prompt.do_cursor(rt, Cursor::up) {
                false => self.prompt.replace_line(self.history.up_recent(), rt),
                _ => {}
            },

            Key::Down => match self.prompt.do_cursor(rt, Cursor::down) {
                false => self.prompt.replace_line(self.history.down_recent(), rt),
                _ => {}
            },

            Key::End => self.prompt.do_cursor(rt, Cursor::to_end),
            Key::Right => {
                if !self.prompt.do_cursor(rt, Cursor::right) {
                    match self.history.guess(&self.prompt.cursor.s) {
                        true => self.prompt.replace_line(self.history.select_recent(0), rt),
                        false => self.prompt.replace_line(None, rt),
                    }
                }
            }
            Key::Left => {
                if !self.prompt.do_cursor(rt, Cursor::left) {
                    self.history.guesses = None;
                    self.prompt.replace_line(None, rt);
                }
            }
            e => println!("{:?}", e),
        }

        Ok(Action::Cont)
    }
}
