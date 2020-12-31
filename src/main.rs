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
        Key::Ctrl('d') => return Ok(Action::Quit),
        Key::Char('\n') => match parser::Lines.parse_s(&sets.line) {
            Ok(v) => {
                rt.suspend_raw_mode().ok();
                print!("\n\r");
                rt.flush().ok();
                for s in v {
                    match s.run(sets) {
                        Ok(true) => print!("\n\rOK - Success\n\r"),
                        Ok(false) => print!("\n\rOK - fail\n\r"),
                        Err(e) => print!("\n\rErr - {}\n\r", e),
                    }
                }
                sets.line.clear();
                ui::print("", rt);
                rt.activate_raw_mode().ok();
            }
            Err(_) => sets.add_char('\n', rt),
        },
        Key::Char('\t') => println!("TODO : TAB"),

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
        Event::Unsupported(c_up) if c_up == [27, 91, 49, 59, 53, 65] => println!("Ctrl UP"),
        e => print!("Event {:?}\n\r", e),
    }
    Ok(Action::Cont)
}

fn main() -> anyhow::Result<()> {
    ctrlc::set_handler(move || println!("Kill Signal")).ok();
    let mut sets = Settings::new();

    let mut rt = stdout().into_raw_mode()?;
    print!("{}> ", termion::cursor::Save);
    rt.flush()?;

    for raw_e in stdin().events_and_raw() {
        let (e, _) = raw_e?;
        match do_event(e, &mut sets, &mut rt) {
            Ok(Action::Quit) => {
                println!("");
                return Ok(());
            }
            Ok(Action::Cont) => {}
            v => print!("Fail : {:?}", v),
        }
    }
    Ok(())
}
