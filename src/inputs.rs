use err_tools::*;
use futures::future::poll_fn;
use std::pin::Pin;
use std::task::Poll;
use tokio::io::*;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub enum Key {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Esc,
    Enter,
    Tab,
    Insert,
    Delete,
    BackSpace,
    End,
}

#[derive(Clone, Debug)]
pub enum Signal {
    Kill,
    Term,
    Stop,
}

#[derive(Clone, Debug)]
pub enum Event {
    Key(Key),
    Ctrl(Key),
    Alt(Key),
    Signal(Signal),
    Unsupported(Vec<u8>),
    Null,
}

pub type REvent = anyhow::Result<Event>;

pub async fn handle_inputs(ch: mpsc::Sender<REvent>) -> anyhow::Result<()> {
    let mut b1: [u8; 1] = [0];
    let mut bbig: [u8; 10] = [0; 10];
    let mut full_buf: Vec<u8> = Vec::new();
    let mut sdin = stdin();
    loop {
        sdin.read(&mut b1).await?;
        let mut bbuf = ReadBuf::new(&mut bbig);
        poll_fn(|c| {
            drop(Pin::new(&mut sdin).poll_read(c, &mut bbuf));
            Poll::Ready(())
        })
        .await;

        full_buf.extend(&b1);
        full_buf.extend(bbuf.filled());

        'inner: loop {
            match parse_event(&full_buf) {
                ParseRes::Ok(e, n) => {
                    drop(full_buf.drain(0..n));
                    //println!("parse : {:?}-{}-{:?}", e, n, full_buf);
                    ch.send(Ok(e)).await?;
                }
                ParseRes::Err(e, n) => {
                    drop(full_buf.drain(0..n));
                    //println!("perr : {:?}-{}-{:?}", e, n, full_buf);
                    ch.send(Err(e)).await?;
                }
                ParseRes::Incomplete => {
                    break 'inner;
                }
            }
        }
    }
}

pub enum ParseRes<T> {
    Ok(T, usize),
    Incomplete,
    Err(anyhow::Error, usize),
}

impl<T> ParseRes<T> {
    pub fn map<F: Fn(T) -> B, B>(self, f: F) -> ParseRes<B> {
        match self {
            ParseRes::Ok(t, n) => ParseRes::Ok(f(t), n),
            ParseRes::Err(e, n) => ParseRes::Err(e, n),
            ParseRes::Incomplete => ParseRes::Incomplete,
        }
    }
}

pub fn parse_event(v: &[u8]) -> ParseRes<Event> {
    if v.len() == 0 {
        return ParseRes::Incomplete;
    }
    match v[0] {
        b'\x1B' => unimplemented! {}, // Control Sequence
        b'\n' | b'\r' => ParseRes::Ok(Event::Key(Key::Enter), 1),
        b'\t' => ParseRes::Ok(Event::Key(Key::Tab), 1),
        b'\x7F' => ParseRes::Ok(Event::Key(Key::BackSpace), 1),
        c @ b'\x01'..=b'\x1A' => ParseRes::Ok(
            Event::Ctrl(Key::Char((c as u8 - b'\x01' + b'a') as char)),
            1,
        ),
        c @ b'\x1C'..=b'\x1F' => ParseRes::Ok(
            Event::Ctrl(Key::Char((c as u8 - b'\x1C' + b'4') as char)),
            1,
        ),
        _ => parse_utf8(v, 0).map(|c| Event::Key(Key::Char(c))),
    }
}

pub fn parse_utf8(v: &[u8], off: usize) -> ParseRes<char> {
    let mut buf: [u8; 4] = [0; 4];
    for x in 0..4 {
        let ox = off + x;
        if ox >= v.len() {
            return ParseRes::Incomplete;
        }
        buf[x] = v[ox];
        if let Ok(s) = std::str::from_utf8(&buf[0..=x]) {
            return ParseRes::Ok(s.chars().next().unwrap(), ox + 1);
        }
    }
    ParseRes::Err(SError("Could not make utf8 Char").into(), 1)
}
