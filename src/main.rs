mod args;
mod channel;
mod cursor;
mod exec;
mod expr;
mod parser;
mod partial;
mod prompt;
mod shell;
mod statement;
mod store;
mod str_util;
mod tab_complete;
mod ui;

use bogobble::traits::*;
use clap::*;
use err_tools::*;
use shell::Shell;
use std::io::*;
use store::Store;
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
        Event::Unsupported(e) => shell.do_unsupported(&e, rt)?,
        e => print!("Event {:?}\n\r", e),
    }
    Ok(Action::Cont)
}

pub fn main() -> anyhow::Result<()> {
    let clp = App::new("Ru Shell")
        .about("A shell with multiline editing and curly syntax")
        .version(crate_version!())
        .author("Matthew Stoodley")
        .subcommand(
            SubCommand::with_name("server").about("The server that manages the command history"),
        )
        .arg(
            (Arg::with_name("fname").index(1).required(false))
                .help("run on a file without going interactive"),
        )
        .get_matches();

    if let Some(_sub) = clp.subcommand_matches("server") {
        return server_main();
    }
    shell_main(&clp)
}

fn server_main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(ru_complete::main());
    Ok(())
}

fn shell_main(clp: &clap::ArgMatches) -> anyhow::Result<()> {
    //TODOsort out args properly
    match clp.value_of("fname") {
        Some(v) => {
            println!("Got file '{}'", v);
            return run_file(v, &mut Store::new()).map(|_| ());
        }
        None => {}
    }

    match termion::is_tty(&stdin()) {
        true => run_interactive(),
        false => run_stream_out(&mut stdin(), &mut Store::new()).map(|_| ()),
    }
}

pub fn run_interactive() -> anyhow::Result<()> {
    ctrlc::set_handler(move || println!("Kill Signal")).ok();
    let mut shell = Shell::new();
    let mut rt = stdout().into_raw_mode()?;

    let mut init = std::path::PathBuf::from(std::env::var("HOME").unwrap_or("".to_string()));
    init.push(".config/rushell/init.rush");

    if let Err(e) = shell.source_path(init) {
        println!("Error sourcing home_config : {}", e);
    }

    shell.reset(&mut rt);

    for raw_e in stdin().events_and_raw() {
        let e = match raw_e {
            Ok((e, _)) => e,
            Err(e) => {
                return e_string(format!("Input Error {}", e));
            }
        };
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

pub fn run_file<P: AsRef<std::path::Path>>(fname: P, store: &mut Store) -> anyhow::Result<bool> {
    let mut f = std::fs::File::open(fname)?;
    run_stream_out(&mut f, store)
}

pub fn run_stream_out<T: std::io::Read>(t: &mut T, store: &mut Store) -> anyhow::Result<bool> {
    let mut s = String::new();
    t.read_to_string(&mut s)?;
    let ar = parser::Lines.parse_s(&s).map_err(|e| e.strung())?;
    store.push();
    crate::statement::run_block_pop(&ar, store)
}
