use crate::args::*;
use crate::channel::*;
use crate::exec::*;
use crate::store::AStore;
use err_tools::*;
use std::process::Stdio;

pub enum Expr {
    Exec(Exec),
    Write {
        exec: Exec,
        chan: Channel,
        filename: Arg,
        append: bool,
    },
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

impl Expr {
    #[async_recursion::async_recursion]
    pub async fn run(&self, s: AStore) -> anyhow::Result<bool> {
        match self {
            Expr::Exec(e) => {
                let mut ch = e
                    .run(&s, Stdio::inherit(), Stdio::inherit(), Stdio::inherit())
                    .await?;
                ch.wait().map(|e| e.success()).map_err(Into::into)
            }
            Expr::Write {
                exec,
                chan,
                filename,
                append,
            } => {
                let filename = filename.run(s.clone()).await?.to_string();
                let ch = exec
                    .run(&s, Stdio::inherit(), Stdio::piped(), Stdio::piped())
                    .await?;
                let mut iread =
                    chan.as_reader(ch.stdout.e_str("No Output")?, ch.stderr.e_str("No ErrPut")?);

                let mut f = std::fs::OpenOptions::new()
                    .append(*append)
                    .truncate(!*append)
                    .write(true)
                    .create(true)
                    .open(filename)?;
                std::io::copy(&mut iread, &mut f)
                    .map(|_| true)
                    .e_str("Could not copy to output file")
            }
            Expr::And(a, b) => match a.run(s.clone()).await {
                Ok(true) => b.run(s).await,
                v => v,
            },
            Expr::Or(a, b) => match a.run(s.clone()).await {
                Ok(false) => b.run(s).await,
                v => v,
            },
        }
    }
}
