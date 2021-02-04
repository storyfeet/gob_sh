use core::task::{Context, Poll};
use std::future::Future;
//use std::marker::Unpin;
use futures::future::poll_fn;
use std::pin::Pin;
use termion::event::Event;
use tokio::io::*;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum AEvent {
    Input(Event),
    KillSignal,
    Error(anyhow::Error),
}

pub async fn handle_inputs(ch: mpsc::Sender<AEvent>) -> anyhow::Result<()> {
    let mut b1: [u8; 1] = [0];
    let mut bbig: [u8; 10] = [0; 10];
    let mut full_buf: Vec<u8> = Vec::new();
    let mut sdin = stdin();
    loop {
        let r1 = sdin.read(&mut b1).await?;
        let mut bbuf = ReadBuf::new(&mut bbig);
        let r2 = poll_fn(|c| {
            drop(Pin::new(&mut sdin).poll_read(c, &mut bbuf));
            Poll::Ready(())
        })
        .await;

        full_buf.extend(&b1);
        full_buf.extend(bbuf.filled());
        while let Some(v) = next_event(&mut full_buf) {
            match v {
                Ok(e) => ch.send(AEvent::Input(e)).await?,
                Err(e) => ch.send(AEvent::Error(e)).await?,
            }
        }
    }
}

pub fn next_event(v: &mut Vec<u8>) -> Option<anyhow::Result<Event>> {
    let a = *v.get(0)?;
    let mut it = v[1..].iter().map(|a| Ok(*a));
    let res = termion::event::parse_event(a, &mut it);
    let v2 = it.filter_map(|a| a.ok()).collect();
    *v = v2;
    Some(res.map_err(|e| e.into()))
}

/*pub enum WaitFor {
    Read,
    Send(AEvent),
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
    fn poll(self: Pin<&mut Self>, ctex: &mut Context<'_>) -> Poll<()> {
        let s = self.get_mut();
        loop {
            match &s.status {
                WaitFor::Read => {
                    let mut b: [u8; 1] = [0];
                    let mut rb = ReadBuf::new(&mut b);
                    match Pin::new(&mut s.sd).poll_read(ctex, &mut rb) {
                        Poll::Ready(Ok(())) => {
                            s.buff.extend(rb.filled());
                            let mut c: [u8; 10] = [0; 10];
                            let mut rc = ReadBuf::new(&mut c);
                            drop(Pin::new(&mut s.sd).poll_read(ctex, &mut rc));
                            s.buff.extend(rc.filled());

                            //TODO work out the rest of the buffer
                        }
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(e)) => s.status = WaitFor::Send(AEvent::Error(e.into())),
                    }
                }
                WaitFor::Send(ae) => match s.ch.try_send(ae.clone()) {
                    Ok(()) => s.status = WaitFor::Read,
                },
            }
        }
    }
}*/
