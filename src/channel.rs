//use crate::settings::Settings;
//use err_tools::*;
//use std::io::Read;
use std::convert::TryInto;
use std::pin::Pin;
use std::process::Stdio;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};
use tokio::process::{ChildStderr, ChildStdout};

pub enum ChannelRead {
    Out(ChildStdout),
    Err(ChildStderr),
    Both(ChildStdout, ChildStderr),
}
/*
impl Read for ChannelRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            ChannelRead::Out(r) => r.read(buf),
            ChannelRead::Err(r) => r.read(buf),
            ChannelRead::Both(o, e) => match o.read(buf) {
                Ok(0) | Err(_) => e.read(buf),
                Ok(n) => Ok(n),
            },
        }
    }
}
*/

impl AsyncRead for ChannelRead {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ChannelRead::Out(ref mut r) => Pin::new(r).poll_read(cx, buf),
            ChannelRead::Err(ref mut r) => Pin::new(r).poll_read(cx, buf),
            ChannelRead::Both(ref mut o, ref mut e) => match Pin::new(o).poll_read(cx, buf) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                _ => Pin::new(e).poll_read(cx, buf),
            },
        }
    }
}

impl ChannelRead {
    pub fn to_stdio(self) -> anyhow::Result<Stdio> {
        Ok(match self {
            ChannelRead::Out(o) => o.try_into()?,
            ChannelRead::Err(o) => o.try_into()?,
            ChannelRead::Both(o, _) => o.try_into()?,
        })
    }
}

#[derive(Clone, Debug)]
pub enum Channel {
    StdOut,
    StdErr,
    Both,
}

impl Channel {
    pub fn as_reader(&self, o: ChildStdout, e: ChildStderr) -> ChannelRead {
        match self {
            Channel::StdOut => ChannelRead::Out(o),
            Channel::StdErr => ChannelRead::Err(e),
            Channel::Both => ChannelRead::Both(o, e),
        }
    }
}
