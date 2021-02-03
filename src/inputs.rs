use core::task::{Context, Poll};
use std::future::Future;
use std::marker::Unpin;
use std::pin::Pin;
use termion::event::Event;
use tokio::io::*;
use tokio::sync::mpsc;

pub enum AEvent {
    Input(Event),
    KillSignal,
    Error(anyhow::Error),
}

pub enum WaitFor {
    Read,
    SendEvent(AEvent),
    SendErr(anyhow::Error),
}

pub struct InputHandle {
    sd: Stdin,
    ch: mpsc::Sender<AEvent>,
    status: WaitFor,
    buff: Vec<u8>,
}

impl InputHandle {
    pub fn new(ch: mpsc::Sender<AEvent>) -> Self {
        InputHandle {
            sd: stdin(),
            ch,
            status: WaitFor::Read,
            buff: Vec::new(),
        }
    }
}

impl Future for InputHandle {
    type Output = ();
    fn poll(self: Pin<&mut Self>, c: &mut Context<'_>) -> Poll<()> {
        let s = self.get_mut();
        loop {
            match s.status {
                WaitFor::Read => {
                    let mut b: [u8; 1] = [0];
                    let mut rb = ReadBuf::new(&mut b);
                    match Pin::new(&mut s.sd).poll_read(c, &mut ReadBuf::new(&mut b)) {
                        Poll::Ready(Ok(())) => {
                            s.buff.extend(rb.filled());
                            //TODO work out the rest of the buffer
                        }
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(e)) => s.status = WaitFor::Err(e),
                    }
                }
                WaitFor::Send => {}
            }
        }
    }
}
