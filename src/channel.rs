//use crate::settings::Settings;
//use err_tools::*;
use std::io::Read;
use std::process::{ChildStderr, ChildStdout, Stdio};

pub enum ChannelRead {
    Out(ChildStdout),
    Err(ChildStderr),
    Both(ChildStdout, ChildStderr),
}

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

impl ChannelRead {
    pub fn to_stdio(self) -> Stdio {
        match self {
            ChannelRead::Out(o) => Stdio::from(o),
            ChannelRead::Err(o) => Stdio::from(o),
            ChannelRead::Both(o, _) => Stdio::from(o),
        }
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
