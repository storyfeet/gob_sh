//! Some options for statements to run, or persistent data
use crate::cursor::Cursor;
use crate::inputs::{Event, Key};
use crate::partial::Item;
use crate::Action;
use bogobble::traits::*;

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

    fn tab_complete(&mut self) {
        self.prompt.clear_help();
        let c_line = &self.prompt.cursor.on_to_space();
        let clen = c_line.len();
        let top = match crate::partial::Lines.parse_s(c_line) {
            Ok(t) => t,
            Err(e) => {
                self.prompt.message = Some(format!("{}", e));
                return;
            }
        };

        let (tabs, tabr) = match top.find_at_end(c_line, |&i| (i == Item::Arg || i == Item::Path)) {
            Some(a) => (a.on_str(c_line), a.range()),
            None => match self.prompt.cursor.is_end() {
                true => ("", bogobble::partial::ranger::Ranger::InEx(clen, clen)),
                false => {
                    self.prompt.message = Some(format!("Could not complete",));
                    return;
                }
            },
        };

        match crate::tab_complete::tab_complete_path(tabs) {
            Complete::None => self.prompt.message = Some(format!("Could not complete '{}'", tabs)),
            Complete::One(tc) => {
                self.prompt.cursor.replace_range(tabr.with_end(clen), &tc);
            }
            Complete::Many(v) => self.prompt.options = Some((tabr.with_end(clen), v)),
        }
    }
    pub fn on_enter(&mut self, rt: &mut RT) {
        let c_line = &self.prompt.cursor.s;
        self.history.pos = None;
        self.history.guesses = None;
        match parser::Lines.parse_s(c_line) {
            Ok(v) => {
                let hist_r = self.history.push_command(c_line.clone());
                if !self.prompt.cursor.is_end() {
                    self.prompt.unprint(rt);
                    self.prompt.print_end(rt);
                }

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
            Err(_) => self.do_print(rt, |sh| sh.prompt.add_char('\n')),
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

    pub fn do_ctrl(&mut self, k: Key, rt: &mut RT) {
        match k {
            Key::Char('d') => { /*handle higher up*/ }
            Key::Char('n') => self.prompt.do_print(rt, |p| p.add_char('\n')),
            Key::Char('h') => self.prompt.do_cursor(rt, Cursor::del_line),
            c => self
                .prompt
                .do_print(rt, |p| p.message = Some(format!("{:?}", c))),
        }
    }

    pub fn do_key(&mut self, k: Key, rt: &mut RT) -> anyhow::Result<Action> {
        match k {
            Key::Char('\n') | Key::Enter => self.on_enter(rt),
            Key::Char('\t') => self.do_print(rt, Shell::tab_complete),
            Key::Char(c) => self.prompt.do_print(rt, |p| p.add_char(c)),
            Key::BackSpace => self.prompt.do_cursor(rt, Cursor::backspace),
            Key::Delete => self.prompt.do_cursor(rt, Cursor::del_char),
            Key::Esc => {
                self.prompt.esc(rt);
                self.history.guesses = None;
            }
            Key::Up => match self.prompt.do_cursor(rt, Cursor::up) {
                false => self.do_print(rt, |p| p.prompt.replace_line(p.history.up_recent())),
                _ => {}
            },

            Key::Down => match self.prompt.do_cursor(rt, Cursor::down) {
                false => self.do_print(rt, |p| p.prompt.replace_line(p.history.down_recent())),
                _ => {}
            },

            Key::End => self.prompt.do_cursor(rt, Cursor::to_line_end),
            Key::Right => {
                if !self.prompt.do_cursor(rt, Cursor::right) {
                    match self.history.guess(&self.prompt.cursor.s) {
                        true => {
                            self.do_print(rt, |s| s.prompt.replace_line(s.history.select_recent(0)))
                        }
                        false => self.prompt.do_print(rt, |p| p.replace_line(None)),
                    }
                }
            }
            Key::Left => {
                if !self.prompt.do_cursor(rt, Cursor::left) {
                    self.history.guesses = None;
                    self.prompt.do_print(rt, |p| p.replace_line(None));
                }
            }
            e => self
                .prompt
                .do_print(rt, |p| p.message = Some(format!("{:?}", e))),
        }

        Ok(Action::Cont)
    }

    pub fn do_unsupported(&mut self, b: &[u8], rt: &mut RT) -> anyhow::Result<()> {
        match b {
            //Ctrl Up:
            [27, 91, 49, 59, 53, 65] => {
                self.do_print(rt, |p| p.prompt.replace_line(p.history.up_recent()))
            }
            //Ctrl End
            [27, 91, 49, 59, 53, 70] => self.prompt.do_cursor(rt, Cursor::to_end),
            c => self.prompt.do_print(rt, |p| {
                p.message = Some(format!("Unsupported Action {:?}", c))
            }),
        }
        Ok(())
    }
    pub fn do_event(&mut self, e: Event, rt: &mut RT) -> anyhow::Result<Action> {
        match e {
            Event::Key(k) => return self.do_key(k, rt),
            Event::Unsupported(e) => self.do_unsupported(&e, rt)?,
            Event::Ctrl(k) => self.do_ctrl(k, rt),
            e => print!("Event {:?}\n\r", e),
        }
        Ok(Action::Cont)
    }
}
