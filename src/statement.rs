use crate::args::{Arg, Args};
use crate::expr::Expr;
use crate::store::{Data, Store};
use err_tools::*;

pub enum Statement {
    Expr(Expr),
    Let(Vec<String>, Args),
    Export(Vec<String>, Args),
    Cd(Arg),
    For {
        vars: Vec<String>,
        args: Args,
        block: Vec<Statement>,
    },
}

impl Statement {
    pub fn run(&self, s: &mut Store) -> anyhow::Result<bool> {
        match self {
            Statement::Expr(e) => e.run(s),
            Statement::Let(names, args) => {
                let ag = args.run(s)?;
                if ag.len() < names.len() {
                    return e_str("Not enough results for var names");
                }
                for (n, k) in names.iter().enumerate() {
                    s.set(k.to_string(), Data::Str(ag[n].clone()))
                }
                Ok(true)
            }
            Statement::Export(names, args) => {
                let ag = args.run(s)?;
                if ag.len() < names.len() {
                    return e_str("Not enough results for var names");
                }
                for (n, k) in names.iter().enumerate() {
                    std::env::set_var(k.to_string(), ag[n].to_string());
                    //                    s.set(k.to_string(), Data::Str(ag[n].clone()))
                }
                Ok(true)
            }
            Statement::Cd(arg) => {
                let mut run_res = arg.run(s)?.to_string();
                if let Some('~') = run_res.chars().next() {
                    let hm = std::env::var("HOME")?;
                    run_res = run_res.replace('~', &hm);
                }

                let s2 = glob::glob(&run_res)?
                    .next()
                    .e_str("Could not glob arg for cd")??;

                std::env::set_current_dir(s2)?;

                std::env::set_var("PWD", std::env::current_dir()?);
                Ok(true)
            }
            Statement::For { vars, args, block } => {
                let ag = args.run(s)?;
                let mut it = ag.into_iter();
                loop {
                    s.push();
                    for vn in vars {
                        let nx = match it.next() {
                            Some(n) => n,
                            None => {
                                s.pop();
                                return Ok(true);
                            }
                        };
                        s.set(vn.to_string(), Data::Str(nx.clone()));
                    }
                    for st in block {
                        st.run(s)?;
                    }

                    s.pop();
                }
            }
        }
    }
}
