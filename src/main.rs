mod args;
mod channel;
mod cursor;
mod exec;
mod expr;
mod future_util;
mod inputs;
mod parser;
mod partial;
mod prompt;
mod shell;
mod statement;
mod store;
mod str_util;
mod tab_complete;
mod ui;
use inputs::{Event, Key};
use tokio::sync::mpsc;

use bogobble::traits::*;
use clap::*;
//use err_tools::*;
use shell::Shell;
use std::io::*;
use store::AStore;
//use termion::event::Event;
//use termion::input::TermReadEventsAndRaw;
use termion::raw::{IntoRawMode, RawTerminal};
use tokio::io::AsyncReadExt;

type RT = RawTerminal<Stdout>;

#[derive(Debug, Clone)]
pub enum Action {
    Cont,
    Quit,
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
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
        //Tab complete server: not yet used in main.
        ru_complete::main().await
    }

    shell_main(&clp).await
}

async fn shell_main<'a>(clp: &'a clap::ArgMatches<'a>) -> anyhow::Result<()> {
    let store = AStore::new_global().await;
    //TODOsort out args properly
    match clp.value_of("fname") {
        Some(v) => {
            println!("Got file '{}'", v);
            return run_file(v, &store).await.map(|_| ());
        }
        None => {}
    }

    match termion::is_tty(&stdin()) {
        true => run_interactive().await,
        false => run_stream_out(&mut tokio::io::stdin(), &store)
            .await
            .map(|_| ()),
    }
}

pub async fn run_interactive() -> anyhow::Result<()> {
    ctrlc::set_handler(move || println!("Kill Signal")).ok();
    let mut shell = Shell::new().await;
    let mut rt = stdout().into_raw_mode()?;

    let mut init = std::path::PathBuf::from(std::env::var("HOME").unwrap_or("".to_string()));
    init.push(".config/rushell/init.rush");
    shell.source_path(init).await;

    shell.reset(&mut rt).await;

    let (ch_s, mut ch_r) = mpsc::channel(10);

    tokio::spawn(inputs::handle_inputs(ch_s));

    while let Some(ae) = ch_r.recv().await {
        match ae {
            Ok(Event::Ctrl(Key::Char('d'))) => return Ok(()),
            Ok(e) => drop(shell.do_event(e, &mut rt).await?),
            Err(e) => shell
                .prompt
                .do_print(&mut rt, |p| p.message = Some(e.to_string())),
        };
    }
    Ok(())
}

pub async fn run_file<P: AsRef<std::path::Path>>(fname: P, store: &AStore) -> anyhow::Result<bool> {
    let mut f = tokio::fs::File::open(fname).await?;
    run_stream_out(&mut f, store).await
}

pub async fn run_stream_out<T: tokio::io::AsyncRead + Unpin>(
    t: &mut T,
    store: &AStore,
) -> anyhow::Result<bool> {
    let mut s = String::new();
    t.read_to_string(&mut s).await?;
    let ar = parser::Lines.parse_s(&s).map_err(|e| e.strung())?;
    crate::statement::run_block(&ar, store).await
}
