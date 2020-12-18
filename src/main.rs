mod args;
mod channel;
mod exec;
mod parser;
mod partial;
mod settings;
mod statement;
mod ui;
use bogobble::traits::*;

//use std::env;
use settings::Settings;
use std::io::*;
use termion::event::Event;
use termion::event::Key;
use termion::input::TermReadEventsAndRaw;
use termion::raw::{IntoRawMode, RawTerminal};

type RT = RawTerminal<Stdout>;

#[derive(Debug, Clone)]
pub enum Action {
    Cont,
    Quit,
}

pub fn do_key(k: Key, sets: &mut Settings, rt: &mut RT) -> anyhow::Result<Action> {
    match k {
        Key::Esc => return Ok(Action::Quit),
        Key::Char(c) => sets.add_char(c, rt),
        Key::Backspace => sets.del_char(rt),
        Key::Ctrl('h') => sets.del_line(rt),
        e => println!("{:?}", e),
    }

    Ok(Action::Cont)
}

pub fn do_event(e: Event, sets: &mut Settings, rt: &mut RT) -> anyhow::Result<Action> {
    match e {
        Event::Key(k) => return do_key(k, sets, rt),
        e => println!("Event {:?}", e),
    }
    Ok(Action::Cont)
}

fn main() -> anyhow::Result<()> {
    let mut sets = Settings::new();

    let mut rt = stdout().into_raw_mode()?;
    print!("{}> ", termion::cursor::Save);
    rt.flush()?;

    for raw_e in stdin().events_and_raw() {
        let (e, _) = raw_e?;
        match do_event(e, &mut sets, &mut rt) {
            Ok(Action::Quit) => return Ok(()),
            Ok(Action::Cont) => {}
            v => print!("Fail : {:?}", v),
        }
    }

    loop {
        print!("> ");
        stdout().flush().ok();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        let statement = match parser::FullStatement.parse_s(&input) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", e);
                continue;
            }
        };
        match statement.run(&mut sets) {
            Ok(true) => println!("\nOK - Success"),
            Ok(false) => println!("\nOK - fail"),
            Err(e) => println!("\nErr - {}", e),
        }
    }
}
