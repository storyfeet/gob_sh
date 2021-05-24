//! Some options for statements to run, or persistent data
use crate::cursor::Cursor;
use crate::partial::Item;
use crate::Action;
use bogobble::traits::*;
use termion::event::Key;

use crate::store::Store;
use crate::tab_complete::*;
use crate::{parser, prompt::Prompt, RT};
use ru_history::HistoryStore;
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
        let mut prompt = Prompt::new(">>".to_string());
        if let Err(e) = load_history(2, &mut history) {
            prompt.message = Some(e.to_string());
        }
        Shell {
            prompt,
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

        let ci = match self.prompt.cursor.item_over() {
            Ok(ci) => ci,
            Err(e) => {
                self.prompt.message = Some(format!("{}", e));
                return;
            }
        };
        let s = ci.on_str(&self.prompt.cursor.s);

        let complete = match ci.item {
            Item::String => tab_complete_path(s),
            Item::Keyword | Item::Ident | Item::Path | Item::Command => {
                self.prompt.message = Some(format!(
                    "Should be able to complete {:?} :'{}'",
                    ci.item,
                    ci.on_str(&self.prompt.cursor.s)
                ));
                return;
            }
            _ => {
                self.prompt.message = Some(format!(
                    "Could not complete {:?} :'{}'",
                    ci.item,
                    ci.on_str(&self.prompt.cursor.s)
                ));
                return;
            }
        };

        match complete {
            Complete::None => self.prompt.message = Some(format!("Could not complete '{}'", s)),
            Complete::One(tc) => {
                self.prompt.cursor.replace_range(ci.to_ranger(), &tc);
            }
            Complete::Many(v) => self.prompt.options = Some((ci.to_ranger(), v)),
        }
    }

    pub fn re_highlight(&mut self) {
        match self.store.get("RU_HIGHLIGHT") {
            Some(s) => self.prompt.set_highlight(&s.to_string()),
            None => self.prompt.set_highlight(""),
        }
    }

    pub fn on_enter(&mut self, rt: &mut RT) {
        let c_line = &self.prompt.cursor.s;
        match parser::Lines.parse_s(c_line) {
            Ok(v) => {
                if v.len() > 0 {
                    self.prompt.guess_man.add_recent(c_line.clone());
                    self.history
                        .add_cmd(&c_line, &ru_history::here(), ru_history::now());
                }
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
                self.re_highlight();

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

    pub fn do_key(&mut self, k: Key, rt: &mut RT) -> anyhow::Result<Action> {
        match k {
            Key::Ctrl('d') => {
                if let Err(_) = save_history(&mut self.history) {
                    println!("Could not save history\n\r");
                }
                return Ok(Action::Quit);
            }
            Key::Char('\n') => self.on_enter(rt),
            Key::Char('\t') => self.do_print(rt, Shell::tab_complete),
            Key::Char(c) => self.prompt.do_print(rt, |p| p.add_char(c)),
            Key::Backspace => self.prompt.do_cursor(rt, Cursor::backspace),
            Key::Delete => self.prompt.do_cursor(rt, Cursor::del_char),
            Key::Ctrl('n') => self.prompt.do_print(rt, |p| p.add_char('\n')),
            Key::Ctrl('h') => self.prompt.do_cursor(rt, Cursor::del_line),
            Key::Esc => {
                self.prompt.esc(rt);
            }
            Key::Up => match self.prompt.do_cursor(rt, Cursor::up) {
                false => self.prompt.do_print(rt, Prompt::up),
                _ => {}
            },
            Key::Down => match self.prompt.do_cursor(rt, Cursor::down) {
                false => self.prompt.do_print(rt, Prompt::down),
                _ => {}
            },
            Key::End => self.prompt.do_cursor(rt, Cursor::to_line_end),
            Key::Right => {
                if !self.prompt.do_cursor(rt, Cursor::right) {
                    let v = self
                        .history
                        .complete(&self.prompt.cursor.s, &ru_history::here(), 16);
                    self.prompt.do_print(rt, move |p| p.set_guesses(v));
                }
            }
            Key::Left => {
                self.prompt.do_cursor(rt, Cursor::left);
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
            [27, 91, 49, 59, 53, 65] => self.prompt.do_print(rt, Prompt::up),
            //Ctrl End
            [27, 91, 49, 59, 53, 70] => self.prompt.do_cursor(rt, Cursor::to_end),
            //Ctrl Down:
            [27, 91, 49, 59, 53, 66] => self.prompt.do_print(rt, Prompt::down),
            c => self.prompt.do_print(rt, |p| {
                p.message = Some(format!("Unsupported Action {:?}", c))
            }),
        }
        Ok(())
    }
}
