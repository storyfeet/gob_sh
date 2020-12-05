use crate::args::Arg;
use crate::exec::{EJoin, Exec};
use crate::statement::Statement as Stt;
use gobble::*;

parser! {(ExJoin ->EJoin)
    or!(
        "|".asv(EJoin::Pipe),
        "^|".asv(EJoin::PipeErr),
    )
}

parser! {(Statement->Stt)
    ExecRight.map(|a|Stt::Exec(a))
}

parser! {(ExecRight->Exec)
    (ExecLeft,maybe((ws__(ExJoin),ExecRight))).map(|(a,op_b)|match op_b{
        Some((jn,b))=>Exec::Join(jn,Box::new(a),Box::new(b)),
        None=>a,
    })
}

parser! {(ExecLeft->Exec)
    (common::Ident , Args).map(|(c,a)|Exec::Simple(c,a))
}

parser! {(Args -> crate::args::Args)
    star(ws_(ArgP)).map(|v| crate::args::Args(v))
}

parser! { (ArgP->Arg)
    or!(
        r_hash.map(|s|Arg::RawString(s) ) ,
        string(star(or_ig!(
                    (Alpha,NumDigit,"?*_.-/").iplus(),
                    ("[",Any.until("]")),
               ))).map(|s|Arg::StringLit(s)),
        common::Quoted.map(|s| Arg::StringLit(s)),
    )
}

/// Raw strings eg: r###" Any \ "##  wierd \ string "###
pub fn r_hash<'a>(it: &LCChars<'a>) -> ParseRes<'a, String> {
    let (it, (_, v, _), _) = ("r", "#".plus(), "\"").parse(it)?;
    Any.until(("\"", "#".exact(v.len())))
        .map(|(s, _)| s)
        .parse(&it)
}
