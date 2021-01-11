mod args;
mod channel;
mod cursor;
mod exec;
mod parser;
mod partial;
mod prompt;
mod shell;
mod statement;
mod store;
mod str_util;
mod tab_complete;
mod ui;

//use std::env;
use shell::Shell;
use std::io::*;
use termion::event::Event;
use termion::input::TermReadEventsAndRaw;
use termion::raw::{IntoRawMode, RawTerminal};

type RT = RawTerminal<Stdout>;

#[derive(Debug, Clone)]
pub enum Action {
    Cont,
    Quit,
}

pub fn do_event(e: Event, shell: &mut Shell, rt: &mut RT) -> anyhow::Result<Action> {
    match e {
        Event::Key(k) => return shell.do_key(k, rt),
        Event::Unsupported(c_up) if c_up == [27, 91, 49, 59, 53, 65] => println!("Ctrl UP"),
        e => print!("Event {:?}\n\r", e),
    }
    Ok(Action::Cont)
}

fn main() -> anyhow::Result<()> {
    ctrlc::set_handler(move || println!("Kill Signal")).ok();
    let mut shell = Shell::new();
    let mut rt = stdout().into_raw_mode()?;

    let mut init = std::path::PathBuf::from(std::env::var("HOME").unwrap_or("".to_string()));
    init.push(".config/rushell/init.rush");

    if let Err(e) = shell.source_path(init) {
        println!("Error sourcing home_config : {}", e);
    }

    shell.reset(&mut rt);

    loop {
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
    }
}
