use crate::args::Args;
use crate::settings::Settings;
use err_tools::*;
use std::process::{Child, Command, Stdio};

#[derive(Clone)]
pub enum EJoin {
    Pipe,
    PipeErr,
}
pub enum Exec {
    Join(EJoin, Box<Exec>, Box<Exec>),
    Simple(String, Args),
}
impl Exec {
    pub fn run(
        &self,
        s: &mut Settings,
        input: Stdio,
        output: Stdio,
        errput: Stdio,
    ) -> anyhow::Result<Child> {
        match self {
            Exec::Simple(name, args) => Command::new(name)
                .args(args.run(s)?)
                .stdin(input)
                .stdout(output)
                .stderr(errput)
                .spawn()
                .map_err(Into::into),
            Exec::Join(EJoin::Pipe, a, b) => {
                let ch = a.run(s, input, Stdio::piped(), Stdio::piped())?;
                b.run(
                    s,
                    Stdio::from(ch.stdout.e_str("No Output from job 'A'")?),
                    output,
                    Stdio::piped(),
                )
            }
            Exec::Join(EJoin::PipeErr, a, b) => {
                let ch = a.run(s, input, Stdio::piped(), Stdio::piped())?;
                b.run(
                    s,
                    Stdio::from(ch.stderr.e_str("No Output from job 'A'")?),
                    output,
                    Stdio::piped(),
                )
            }
        }
    }
}
