//! Some options for statements to run, or persistent data
use crate::partial::Item;
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
}

impl Shell {
    /// Invariants : Settings must always have at least one layer in scope.
    pub fn new() -> Shell {
        Shell {
            prompt: Prompt::new(">>".to_string()),
            store: Store::new(),
        }
    }

    pub fn tab_complete(&mut self, rt: &mut RT) -> anyhow::Result<()> {
        self.prompt.unprint(rt);
        let a = self._tab_complete();
        self.prompt.print(rt);
        a
    }

    fn _tab_complete(&mut self) -> anyhow::Result<()> {
        let top = crate::partial::Lines
            .parse_s(&self.prompt.line)
            .map_err(|e| e.strung())?;

        if let Some(a) = top.find_at_end(&self.prompt.line, |&i| i == Item::Arg) {
            let s = a.on_str(&self.prompt.line);
            match crate::tab_complete::tab_complete_path(s) {
                Complete::None => {}
                Complete::One(tc) => self.prompt.line.replace_range(a.start.unwrap_or(0).., &tc),
                Complete::Many(_v) => println!("Multiple optione"),
            }
        }

        Ok(())
    }
    pub fn on_enter(&mut self, rt: &mut RT) {
        match parser::Lines.parse_s(&self.prompt.line) {
            Ok(v) => {
                rt.suspend_raw_mode().ok();
                print!("\n\r");
                rt.flush().ok();
                for s in v {
                    match s.run(&mut self.store) {
                        Ok(true) => print!("\n\rOK - Success\n\r"),
                        Ok(false) => print!("\n\rOK - fail\n\r"),
                        Err(e) => print!("\n\rErr - {}\n\r", e),
                    }
                }
                rt.activate_raw_mode().ok();
                self.reset(rt);
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
}
