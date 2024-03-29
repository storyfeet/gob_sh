use crate::args::Args;
use crate::channel::*;
use crate::store::Store;
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
        sets: &mut Store,
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
        s: &mut Store,
        input: Stdio,
        output: Stdio,
        errput: Stdio,
    ) -> anyhow::Result<Child> {
        match &self.conn {
            None => Command::new(&self.command)
                .args(self.args.run_s_vec(s, 3)?)
                .stdin(input)
                .stdout(output)
                .stderr(errput)
                .spawn()
                .e_string(format!("Error running {}", &self.command)),
            //.map_err(Into::into),
            Some(conn) => {
                let ch = Command::new(&self.command)
                    .args(self.args.run_s_vec(s, 3)?)
                    .stdin(input)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .e_string(format!("Error running {}", &self.command))?;
                conn.run(ch, s, output, errput)
            }
        }
    }

    pub fn disown(&self) -> anyhow::Result<u32> {
        let ch = self.run(
            &mut Store::new(),
            Stdio::null(),
            Stdio::null(),
            Stdio::null(),
        )?;
        Ok(ch.id())
    }
}
