use crate::args::Args;
use crate::exec::Exec;
use crate::expr::Expr;
use crate::store::{Data, Store};
use err_tools::*;

pub enum Statement {
    Expr(Expr),
    Assign(&'static str, Vec<String>, Args),
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
    Builtin(&'static str, Args),
}

impl Statement {
    pub fn run(&self, s: &mut Store) -> anyhow::Result<bool> {
        match self {
            Statement::Expr(e) => {
                //println!("Running Expr:{:?}", e);
                e.run(s)
            }
            Statement::Assign("let", names, args) => {
                let ag = args.run_data(s)?;
                if ag.len() < names.len() {
                    return e_str("Not enough results for var names");
                }
                for (n, k) in names.iter().enumerate() {
                    s.set(k.to_string(), ag[n].clone())
                }
                Ok(true)
            }
            Statement::Assign("export", names, args) => {
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
            Statement::Assign(a, _, _) => {
                println!("Assigner doesn't exist : '{}'", a);
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
                    run_block_pop(&block, s)?;
                }
            }
            Statement::If { expr, block, else_ } => match expr.run(s)? {
                true => {
                    s.push();
                    run_block_pop(&block, s)
                }
                false => match &else_ {
                    Some(ee) => {
                        s.push();
                        run_block_pop(&ee, s)
                    }
                    None => Ok(true),
                },
            },
            Statement::Disown(e) => {
                let id = e.disown()?;
                println!("PID = {}", id);
                Ok(true)
            }
            Statement::Builtin("cd", args) => {
                let mut run_res = match args.run(s)?.get(0) {
                    Some(v) => v.to_string(),
                    None => {
                        let hm = std::env::var("HOME")?;
                        std::env::set_current_dir(hm)?;
                        std::env::set_var("PWD", std::env::current_dir()?);
                        return Ok(true);
                    }
                };
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
            Statement::Builtin("load", args) => {
                //println!("Running builtin '.'");
                let ag = args.run(s)?;
                for a in ag {
                    println!("Loading {}", a);
                    crate::run_file(a, s)?;
                }
                Ok(true)
            }
            Statement::Builtin("proglist", args) => {
                let ag = args.run(s)?;
                for a in ag {
                    let matches = crate::tab_complete::prog_matches(&a);
                    for m in matches {
                        println!("--{}", m);
                    }
                }
                Ok(true)
            }
            Statement::Builtin(b, _) => {
                println!("Builtin doesn't exist : '{}'", b);
                Ok(true)
            }
        }
    }
}

pub fn run_block_pop(block: &[Statement], store: &mut Store) -> anyhow::Result<bool> {
    for st in block {
        match st.run(store) {
            Ok(_) => {}
            Err(e) => {
                store.pop();
                return Err(e);
            }
        }
    }
    store.pop();
    Ok(true)
}
