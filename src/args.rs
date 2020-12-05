use crate::exec::Exec;
use crate::statement::Settings;

pub struct Args(pub Vec<Arg>);

impl Args {
    pub fn run(&self, _: &mut Settings) -> anyhow::Result<Vec<String>> {
        let mut res = Vec::new();
        for a in &self.0 {
            match a {
                Arg::RawString(s) => res.push(s.to_string()),
                Arg::StringLit(s) => match glob::glob(s) {
                    Ok(v) => {
                        let mut found = false;
                        for a in v {
                            found = true;
                            if let Ok(val) = a {
                                res.push(val.display().to_string());
                            }
                        }
                        if !found {
                            res.push(s.to_string());
                        }
                    }
                    Err(_) => {
                        res.push(s.to_string());
                    }
                },
                _ => {
                    unimplemented! {}
                }
            }
        }
        Ok(res)
    }
}

pub enum Arg {
    RawString(String),
    StringLit(String),
    StringExpr(Vec<StringPart>),
    Command(Exec),
}

pub enum StringPart {
    Lit(String),
    Var(String),
    Command(Exec),
}
