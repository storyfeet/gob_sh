use crate::args::Args;
use crate::settings::Settings;
use err_tools::*;
use std::process::{Child, Command, Stdio};

#[derive(Clone, Debug)]
pub enum Channel {
    StdOut,
    StdErr,
}

#[derive(Clone, Debug)]
pub enum Target {
    Exec(Box<Exec>),
    Append(String),
    Write(String),
}
#[derive(Clone, Debug)]
pub struct Connection {
    pub chan: Channel,
    pub target: Target,
}

impl Connection {
    pub fn run(
        &self,
        ch: Child,
        sets: &mut Settings,
        out: Stdio,
        err: Stdio,
    ) -> anyhow::Result<Child> {
        let input = match self.chan {
            Channel::StdOut => Stdio::from(ch.stdout.e_str("No Out Channel")?),
            Channel::StdErr => Stdio::from(ch.stderr.e_str("No Err Channel")?),
        };

        match &self.target {
            Target::Exec(e) => e.run(sets, input, out, err),
            _ => unimplemented! {},
        }
    }
}

#[derive(Clone, Debug)]
pub struct Exec {
    pub command: String,
    pub args: Args,
    pub conn: Option<Connection>,
}
impl Exec {
    pub fn run(
        &self,
        s: &mut Settings,
        input: Stdio,
        output: Stdio,
        errput: Stdio,
    ) -> anyhow::Result<Child> {
        match &self.conn {
            None => Command::new(&self.command)
                .args(self.args.run(s)?)
                .stdin(input)
                .stdout(output)
                .stderr(errput)
                .spawn()
                .map_err(Into::into),

            Some(conn) => {
                let ch = Command::new(&self.command)
                    .args(self.args.run(s)?)
                    .stdin(input)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;
                conn.run(ch, s, output, errput)
            }
        }
    }
}
