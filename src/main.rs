mod args;
mod channel;
mod exec;
mod parser;
mod partial;
mod prompt;
mod shell;
mod statement;
mod store;
mod tab_complete;
mod ui;

//use std::env;
use shell::Shell;
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

pub fn do_key(k: Key, sets: &mut Shell, rt: &mut RT) -> anyhow::Result<Action> {
    match k {
        Key::Ctrl('d') => return Ok(Action::Quit),
        Key::Char('\n') => sets.on_enter(rt),
        Key::Char('\t') => sets.tab_complete(rt).expect("PROBLEM with TABBING"),

        Key::Char(c) => sets.prompt.add_char(c, rt),
        Key::Backspace => sets.prompt.del_char(rt),
        Key::Ctrl('h') => sets.prompt.del_line(rt),
        e => println!("{:?}", e),
    }

    Ok(Action::Cont)
}

pub fn do_event(e: Event, sets: &mut Shell, rt: &mut RT) -> anyhow::Result<Action> {
    match e {
        Event::Key(k) => return do_key(k, sets, rt),
        Event::Unsupported(c_up) if c_up == [27, 91, 49, 59, 53, 65] => println!("Ctrl UP"),
        e => print!("Event {:?}\n\r", e),
    }
    Ok(Action::Cont)
}

fn main() -> anyhow::Result<()> {
    ctrlc::set_handler(move || println!("Kill Signal")).ok();
    let mut rt = stdout().into_raw_mode()?;
    let mut shell = Shell::new();

    let mut init = std::path::PathBuf::from(std::env::var("HOME").unwrap_or("".to_string()));
    init.push(".config/rushell/init.rush");

    if let Err(e) = shell.source_path(init) {
        println!("Error sourcing home_config : {}", e);
    }

    shell.reset(&mut rt);

    for raw_e in stdin().events_and_raw() {
        let (e, _) = raw_e?;
        match do_event(e, &mut shell, &mut rt) {
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
