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
    BackTab,
    Insert,
    Delete,
    BackSpace,
    End,
    Home,
    F(u8),
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
    MouseEvent(u8, u16, u16),
}

pub type REvent = anyhow::Result<Event>;

macro_rules! b_at {
    ($b:expr,$a:expr) => {
        match $b.get($a) {
            Some(a) => *a,
            None => return ParseRes::Incomplete,
        }
    };
}

macro_rules! p_try {
    ($e:expr) => {
        match $e {
            ParseRes::Ok(v, n) => (v, n),
            ParseRes::Err(e, n) => return ParseRes::Err(e, n),
            ParseRes::Incomplete => return ParseRes::Incomplete,
        }
    };
}

pub async fn handle_inputs(ch: mpsc::Sender<REvent>) -> anyhow::Result<()> {
    let mut b1: [u8; 1] = [0];
    let mut bbig: [u8; 4] = [0; 4];
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
    match b_at!(v, 0) {
        b'\x1B' => {
            match b_at!(v, 1) {
                b'O' => match b_at!(v, 2) {
                    val @ b'P'..=b'S' => ParseRes::Ok(Event::Key(Key::F(1 + val - b'P')), 3),
                    b => ParseRes::Err(SgError(format!("Unexpected '{}'", b)).into(), 2),
                },
                b'[' => parse_csi(v, 2), // Control Sequence
                _ => parse_utf8(v, 1).map(|c| Event::Alt(Key::Char(c))),
            }
        }
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

fn parse_utf8(v: &[u8], off: usize) -> ParseRes<char> {
    let mut buf: [u8; 4] = [0; 4];
    for x in 0..4 {
        let ox = off + x;
        buf[x] = b_at!(v, ox);
        if let Ok(s) = std::str::from_utf8(&buf[0..=x]) {
            return ParseRes::Ok(s.chars().next().unwrap(), ox + 1);
        }
    }
    ParseRes::Err(SError("Could not make utf8 Char").into(), 1)
}

fn parse_csi(v: &[u8], off: usize) -> ParseRes<Event> {
    match b_at!(v, off) {
        b'[' => {
            return match b_at!(v, off + 1) {
                val @ b'A'..=b'E' => ParseRes::Ok(Event::Key(Key::F(1 + val - b'A')), off + 2),
                _ => ParseRes::Err(SError("wierd after [").into(), off + 2),
            }
        }
        b'D' => ParseRes::Ok(Event::Key(Key::Left), off + 1),
        b'C' => ParseRes::Ok(Event::Key(Key::Right), off + 1),
        b'A' => ParseRes::Ok(Event::Key(Key::Up), off + 1),
        b'B' => ParseRes::Ok(Event::Key(Key::Down), off + 1),
        b'H' => ParseRes::Ok(Event::Key(Key::Home), off + 1),
        b'F' => ParseRes::Ok(Event::Key(Key::End), off + 1),
        b'Z' => ParseRes::Ok(Event::Key(Key::BackTab), off + 1),
        b'M' => ParseRes::Ok(
            Event::MouseEvent(
                b_at!(v, off + 1),
                b_at!(v, off + 2) as u16,
                b_at!(v, off + 3) as u16,
            ),
            off + 4,
        ),
        b'<' => {
            let (cb, off) = match p_try!(parse_to_m_semi(v, off + 1)) {
                ((cb, true), off) => (cb, off),
                ((cb, false), off) => return ParseRes::Ok(Event::MouseEvent(cb as u8, 0, 0), off),
            };
            let (cx, off) = match p_try!(parse_to_m_semi(v, off + 1)) {
                ((cx, true), off) => (cx, off),
                ((cx, false), off) => return ParseRes::Ok(Event::MouseEvent(cb as u8, cx, 0), off),
            };
            match p_try!(parse_to_m_semi(v, off + 1)) {
                ((_, true), off) => {
                    ParseRes::Err(SError("Too many args for mouse event").into(), off)
                }
                ((cy, false), off) => ParseRes::Ok(Event::MouseEvent(cb as u8, cx, cy), off),
            }
        }
        _ => unimplemented! {},
    }
}

fn parse_to_m_semi(v: &[u8], off: usize) -> ParseRes<(u16, bool)> {
    let mut res: u16 = 0;
    for i in 0..6 {
        match b_at!(v, off + i) {
            c @ b'0'..=b'9' => res = res * 10 + ((c - b'0') as u16),
            b' ' | b'\t' => {}
            b';' => return ParseRes::Ok((res, true), off + i + 1),
            b'm' | b'M' => return ParseRes::Ok((res, false), off + i + 1),
            _ => return ParseRes::Err(SError("Unexpected Char in u16").into(), off + i),
        }
    }
    ParseRes::Err(SError("Xterm Mouse Event Error").into(), off + 6)
}
