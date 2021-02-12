use crate::args::{Arg, Args};
use crate::exec::Exec;
use crate::expr::Expr;
use crate::store::{AStore, Data};
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
    If {
        expr: Expr,
        block: Vec<Statement>,
        else_: Option<Vec<Statement>>,
    },
    Disown(Exec),
    Dot(String),
}

impl Statement {
    pub async fn run(&self, s: &AStore) -> anyhow::Result<bool> {
        match self {
            Statement::Expr(e) => e.run(s.clone()).await,
            Statement::Let(names, args) => {
                let ag = args.run(&s).await?;
                if ag.len() < names.len() {
                    return e_str("Not enough results for var names");
                }
                for (n, k) in names.iter().enumerate() {
                    s.set(k.to_string(), Data::Str(ag[n].clone())).await
                }
                Ok(true)
            }
            Statement::Export(names, args) => {
                let ag = args.run(s).await?;
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
                let mut run_res = arg.run(s.clone()).await?.to_string();
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
                let ag = args.run(s).await?;
                let mut it = ag.into_iter();
                loop {
                    for vn in vars {
                        let nx = match it.next() {
                            Some(n) => n,
                            None => {
                                return Ok(true);
                            }
                        };
                        s.set(vn.to_string(), Data::Str(nx.clone())).await;
                    }
                    run_block(&block, &s.child().await).await?;
                }
            }
            Statement::If { expr, block, else_ } => match expr.run(s.clone()).await? {
                true => run_block(&block, &s.child().await).await,
                false => match &else_ {
                    Some(ee) => run_block(&ee, &s.child().await).await,

                    None => Ok(true),
                },
            },
            Statement::Disown(e) => {
                let id = e.disown().await?;
                println!("PID = {}", id);
                Ok(true)
            }
            Statement::Dot(p) => crate::run_file(p, s).await,
        }
    }
}

#[async_recursion::async_recursion]
pub async fn run_block(block: &[Statement], store: &AStore) -> anyhow::Result<bool> {
    for st in block {
        match st.run(store).await {
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(true)
}
