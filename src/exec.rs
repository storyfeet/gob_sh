use crate::args::Args;
use crate::channel::*;
use crate::store::AStore;
use err_tools::*;
//use std::convert::TryInto;
use std::process::Stdio;
use tokio::process::{Child, Command};

#[derive(Clone, Debug)]
pub struct Connection {
    pub chan: Channel,
    pub target: Box<Exec>,
}

impl Connection {
    #[async_recursion::async_recursion]
    pub async fn run(
        &self,
        ch: Child,
        sets: AStore,
        out: Stdio,
        err: Stdio,
    ) -> anyhow::Result<Child> {
        let iread = self
            .chan
            .as_reader(ch.stdout.e_str("no output")?, ch.stderr.e_str("no errput")?);

        self.target.run(&sets, iread.to_stdio()?, out, err).await
    }
}

#[derive(Clone, Debug)]
pub struct Exec {
    pub command: String,
    pub args: Args,
    pub conn: Option<Connection>,
}

impl Exec {
    pub async fn run(
        &self,
        s: &AStore,
        input: Stdio,
        output: Stdio,
        errput: Stdio,
    ) -> anyhow::Result<Child> {
        match &self.conn {
            None => Command::new(&self.command)
                .args(self.args.run(s).await?)
                .stdin(input)
                .stdout(output)
                .stderr(errput)
                .spawn()
                .map_err(Into::into),

            Some(conn) => {
                let ch = Command::new(&self.command)
                    .args(self.args.run(s).await?)
                    .stdin(input)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;
                conn.run(ch, s.clone(), output, errput).await
            }
        }
    }

    pub async fn disown(&self) -> anyhow::Result<u32> {
        let ch = self
            .run(
                &mut AStore::new_global().await,
                Stdio::null(),
                Stdio::null(),
                Stdio::null(),
            )
            .await?;
        ch.id().e_str("No ID on process")
    }
}
