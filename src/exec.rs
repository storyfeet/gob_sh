use crate::args::Args;
use crate::channel::*;
use crate::settings::Settings;
use err_tools::*;
use std::process::{Child, Command, Stdio};

#[derive(Clone, Debug)]
pub struct Connection {
    pub chan: Channel,
    pub target: Box<Exec>,
}

impl Connection {
    pub fn run(
        &self,
        ch: Child,
        sets: &mut Settings,
        out: Stdio,
        err: Stdio,
    ) -> anyhow::Result<Child> {
        let iread = self
            .chan
            .as_reader(ch.stdout.e_str("No output")?, ch.stderr.e_str("No errput")?);

        self.target.run(sets, iread.to_stdio(), out, err)
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
