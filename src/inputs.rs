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
    CSI(Vec<u16>, u8),
    Prompt(String),
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

macro_rules! at_safe {
    ($v:expr,$a:expr) => {
        at_safe!($v, $a, 0)
    };
    ($v:expr,$a:expr,$def:expr) => {
        match $v.len() > $a {
            true => $v[$a],
            false => $def,
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

pub struct InByteReader {
    buf: [u8; 6],
    start: usize,
    end: usize,
}

impl InByteReader {
    pub fn new() -> Self {
        InByteReader {
            buf: [0; 6],
            start: 0,
            end: 0,
        }
    }
    async fn next_byte(&mut self) -> u8 {
        println!("next_byte {},{},{:?}\r", self.start, self.end, self.buf);
        if self.end > self.start {
            self.start += 1;
            println!("b = {}", self.buf[self.start - 1]);
            return self.buf[self.start - 1];
        }
        let mut sin = stdin();
        sin.read(&mut self.buf[..1]).await.ok();
        let mut bbuf = ReadBuf::new(&mut self.buf[1..3]);
        poll_fn(|c| {
            println!("poll_fn_start: {:?}\r", bbuf.filled());
            match Pin::new(&mut sin).poll_read(c, &mut bbuf) {
                Poll::Ready(_) => println!("filled buf = {:?}\r", bbuf.filled()),
                Poll::Pending => println!("no fill buf = {:?}\r", bbuf.filled()),
            }
            Poll::Ready(())
        })
        .await;
        self.end = 1 + bbuf.filled().len();
        self.start = 1;
        println!("next_byte_end {},{},{:?}\r", self.start, self.end, self.buf);
        self.buf[0]
    }

    async fn try_next_byte(&mut self) -> Option<u8> {
        println!("try_next_byte {},{},{:?}\r", self.start, self.end, self.buf);
        if self.start < self.end {
            self.start += 1;
            return Some(self.buf[self.start - 1]);
        }
        let mut sin = stdin();
        let mut bbuf = ReadBuf::new(&mut self.buf[..1]);
        let ok = poll_fn(|c| match Pin::new(&mut sin).poll_read(c, &mut bbuf) {
            Poll::Ready(_) => Poll::Ready(true),
            Poll::Pending => Poll::Ready(false),
        })
        .await;
        match ok {
            false => None,
            true => Some(self.buf[0]),
        }
    }
}

pub struct EventReader {
    bt: InByteReader,
}

impl EventReader {
    pub fn new() -> Self {
        Self {
            bt: InByteReader::new(),
        }
    }
    pub async fn next_event(&mut self) -> anyhow::Result<Event> {
        match self.bt.next_byte().await {
            b'\x1B' => {
                match self.bt.try_next_byte().await {
                    None => Ok(Event::Key(Key::Esc)),
                    Some(b'O') => match self.bt.next_byte().await {
                        val @ b'P'..=b'S' => Ok(Event::Key(Key::F(1 + val - b'P'))),
                        b => Err(SgError(format!("Unexpected '{}'", b)).into()),
                    },
                    Some(b'[') => self.parse_csi().await, // Control Sequence
                    _ => self.parse_utf8().await.map(|c| Event::Alt(Key::Char(c))),
                }
            }
            b'\n' | b'\r' => Ok(Event::Key(Key::Enter)),
            b'\t' => Ok(Event::Key(Key::Tab)),
            b'\x7F' => Ok(Event::Key(Key::BackSpace)),
            c @ b'\x01'..=b'\x1A' => Ok(Event::Ctrl(Key::Char((c as u8 - b'\x01' + b'a') as char))),
            c @ b'\x1C'..=b'\x1F' => Ok(Event::Ctrl(Key::Char((c as u8 - b'\x1C' + b'4') as char))),
            _ => self.parse_utf8().await.map(|c| Event::Key(Key::Char(c))),
        }
    }
    async fn parse_utf8(&mut self) -> anyhow::Result<char> {
        let mut buf: [u8; 4] = [0; 4];
        for x in 0..4 {
            buf[x] = self.bt.next_byte().await;
            if let Ok(s) = std::str::from_utf8(&buf[0..=x]) {
                return Ok(s.chars().next().unwrap());
            }
        }
        Err(SError("Could not make utf8 Char").into())
    }

    async fn parse_csi(&mut self) -> anyhow::Result<Event> {
        match self.bt.next_byte().await {
            b'[' => {
                return match self.bt.next_byte().await {
                    val @ b'A'..=b'E' => Ok(Event::Key(Key::F(1 + val - b'A'))),
                    _ => Err(SError("wierd after [").into()),
                }
            }
            b'D' => Ok(Event::Key(Key::Left)),
            b'C' => Ok(Event::Key(Key::Right)),
            b'A' => Ok(Event::Key(Key::Up)),
            b'B' => Ok(Event::Key(Key::Down)),
            b'H' => Ok(Event::Key(Key::Home)),
            b'F' => Ok(Event::Key(Key::End)),
            b'Z' => Ok(Event::Key(Key::BackTab)),
            b'M' => {
                let a = self.bt.next_byte().await;
                let b = self.bt.next_byte().await;
                let c = self.bt.next_byte().await;
                Ok(Event::MouseEvent(a, b as u16, c as u16))
            }
            b'<' => {
                let (dat, _) = self.csi_data().await?;

                Ok(Event::MouseEvent(
                    at_safe!(dat, 0) as u8,
                    at_safe!(dat, 1),
                    at_safe!(dat, 2),
                ))
            }
            _c @ b'0'..=b'9' => self.csi_data().await.map(|(a, n)| Event::CSI(a, n)),
            _ => Ok(Event::Null),
        }
    }

    async fn csi_data(&mut self) -> anyhow::Result<(Vec<u16>, u8)> {
        let mut res = Vec::new();

        loop {
            let (n, s) = self.parse_to_m_semi().await?;
            res.push(n);
            match s {
                b';' => {}
                c => return Ok((res, c)),
            }
        }
    }

    async fn parse_to_m_semi(&mut self) -> anyhow::Result<(u16, u8)> {
        let mut res: u16 = 0;
        for _ in 0..6 {
            match self.bt.next_byte().await {
                c @ b'0'..=b'9' => res = res * 10 + ((c - b'0') as u16),
                b' ' | b'\t' => {}
                b';' => return Ok((res, b';')),
                c @ 64..=126 => return Ok((res, c)),
                _ => return Err(SError("Unexpected Char in u16").into()),
            }
        }
        Err(SError("Xterm Mouse Event Error").into())
    }
}

pub async fn handle_inputs(ch: mpsc::Sender<REvent>) -> anyhow::Result<()> {
    let mut ir = EventReader::new();
    loop {
        /* match ir.next_event().await {
            Ok(ev) => ch.send(Ok(ev)).await.ok(),
            Err(e) => ch.send(Err(e)).await.ok(),
        };*/
        println!("EVENT:{:?}\r", ir.next_event().await)
    }
}

pub async fn handle_inputs_old(ch: mpsc::Sender<REvent>) -> anyhow::Result<()> {
    let mut b1: [u8; 1] = [0];
    let mut bbig: [u8; 20] = [0; 20];
    let mut full_buf: Vec<u8> = Vec::new();
    let mut sdin = stdin();
    let mut needed = true;
    loop {
        if needed {
            sdin.read(&mut b1).await?;
            full_buf.extend(&b1);
            needed = false;
        }

        'grabber: for n in 0..6 {
            let mut bbuf = ReadBuf::new(&mut bbig[0..(1 + 3 * n)]);
            poll_fn(|c| {
                drop(Pin::new(&mut sdin).poll_read(c, &mut bbuf));
                Poll::Ready(())
            })
            .await;
            if bbuf.filled().is_empty() {
                break 'grabber;
            }
            full_buf.extend(bbuf.filled());
        }

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
                needed = true;
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
            match v.get(1).map(|a| *a) {
                None => ParseRes::Ok(Event::Key(Key::Esc), 1),
                Some(b'O') => match b_at!(v, 2) {
                    val @ b'P'..=b'S' => ParseRes::Ok(Event::Key(Key::F(1 + val - b'P')), 3),
                    b => ParseRes::Err(SgError(format!("Unexpected '{}'", b)).into(), 2),
                },
                Some(b'[') => parse_csi(v, 2), // Control Sequence
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
            let ((dat, _), off) = p_try!(csi_data(v, off + 1));

            ParseRes::Ok(
                Event::MouseEvent(at_safe!(dat, 0) as u8, at_safe!(dat, 1), at_safe!(dat, 2)),
                off,
            )
        }
        _c @ b'0'..=b'9' => csi_data(v, off).map(|(a, n)| Event::CSI(a, n)),
        _ => ParseRes::Ok(Event::Null, off + 1),
    }
}

fn csi_data(v: &[u8], mut off: usize) -> ParseRes<(Vec<u16>, u8)> {
    let mut res = Vec::new();

    loop {
        let ((n, s), noff) = p_try!(parse_to_m_semi(v, off));
        res.push(n);
        match s {
            b';' => off = noff,
            c => return ParseRes::Ok((res, c), noff),
        }
    }
}

fn parse_to_m_semi(v: &[u8], off: usize) -> ParseRes<(u16, u8)> {
    let mut res: u16 = 0;
    for i in 0..6 {
        match b_at!(v, off + i) {
            c @ b'0'..=b'9' => res = res * 10 + ((c - b'0') as u16),
            b' ' | b'\t' => {}
            b';' => return ParseRes::Ok((res, b';'), off + i + 1),
            c @ 64..=126 => return ParseRes::Ok((res, c), off + i + 1),
            _ => return ParseRes::Err(SError("Unexpected Char in u16").into(), off + i),
        }
    }
    ParseRes::Err(SError("Xterm Mouse Event Error").into(), off + 6)
}
