//! Some options for statements to run, or persistent data
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
        Shell {
            prompt: Prompt::new(">>".to_string()),
            store: Store::new(),
            history: HistoryStore::new(),
        }
    }

    pub fn tab_complete(&mut self, rt: &mut RT) {
        self.prompt.unprint(rt);
        self._tab_complete().ok();
        self.prompt.print(rt);
    }

    fn _tab_complete(&mut self) -> anyhow::Result<()> {
        let top = crate::partial::Lines
            .parse_s(&self.prompt.line)
            .map_err(|e| e.strung())?;
        self.prompt.clear_help();

        if let Some(a) = top.find_at_end(&self.prompt.line, |&i| i == Item::Arg) {
            let s = a.on_str(&self.prompt.line);

            match crate::tab_complete::tab_complete_path(s) {
                Complete::None => self.prompt.message = Some(format!("Could not complete '{}'", s)),
                Complete::One(tc) => {
                    self.prompt.line.replace_range(a.start.unwrap_or(0).., &tc);
                }
                Complete::Many(v) => self.prompt.options = Some((a.start.unwrap_or(0), v)),
            }
        }

        Ok(())
    }
    pub fn on_enter(&mut self, rt: &mut RT) {
        self.history.pos = None;
        self.history.guesses = None;
        match parser::Lines.parse_s(&self.prompt.line) {
            Ok(v) => {
                let hist_r = self.history.push_command(self.prompt.line.clone());

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
                    Ok(s) => self.prompt.message = Some(s),
                }
                self.prompt.print(rt);
            }
            Err(_) => self.prompt.add_char('\n', rt),
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
            Key::Char('\t') => self.tab_complete(rt),

            Key::Char(c) => self.prompt.add_char(c, rt),
            Key::Backspace => self.prompt.del_char(rt),
            Key::Ctrl('h') => self.prompt.del_line(rt),
            Key::Esc => self.prompt.esc(rt),
            Key::Up => self.prompt.replace_line(self.history.up_recent(), rt),
            Key::Down => self.prompt.replace_line(self.history.down_recent(), rt),
            Key::Right => match self.history.guess(&self.prompt.line) {
                true => self.prompt.replace_line(self.history.select_recent(0), rt),
                false => self.prompt.replace_line(None, rt),
            },
            Key::Left => {
                self.history.guesses = None;
                self.prompt.replace_line(None, rt);
            }
            e => println!("{:?}", e),
        }

        Ok(Action::Cont)
    }
}
