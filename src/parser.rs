use crate::args::Arg;
use crate::exec::{EJoin, Exec};
use crate::statement::Statement as Stt;
use gobble::*;

parser! {(End->())
    or_ig!("\n;".one(),EOI)
}

parser! {(ExJoin ->EJoin)
    or!(
        "|".asv(EJoin::Pipe),
        "^|".asv(EJoin::PipeErr),
    )
}

parser! {(FullStatement->Stt)
    first(Statement,End)
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

parser! { (QuotedLitString->String)
    strings_plus(or!(
            not("${}()[]\\\"").plus(),
            "\\n".map(|_|"\n".to_string()),
            "\\t".map(|_|"\t".to_string()),
             ("\\",Any.one()).map(|(_,c)| {let mut s=String::new(); s.push(c);s}),
    ))
}

parser! { (LitString->String)
    strings_plus(or!(
            not("${}()[]\\\" \n\t").plus(),
            "\\n".map(|_|"\n".to_string()),
            "\\t".map(|_|"\t".to_string()),
             ("\\",Any.one()).map(|(_,c)| {let mut s=String::new(); s.push(c);s}),
    ))
}

parser! {(StringPart->Arg)
    or!(
        ("$",(Alpha,NumDigit,"_").plus()).map(|(_,s)|Arg::Var(s)),
        ("${",ws__((Alpha,NumDigit,"_").plus()),"}").map(|(_,s,_)|Arg::Var(s)),
        ("(",ws__(ExecLeft),")").map(|(_,e,_)|Arg::Command(e)),
        LitString.map(|s|Arg::StringLit(s)),
    )
}

parser! {(QuotedStringPart->Arg)
    or!(
        ("$",(Alpha,NumDigit,"_").plus()).map(|(_,s)|Arg::Var(s)),
        ("${",ws__((Alpha,NumDigit,"_").plus()),"}").map(|(_,s,_)|Arg::Var(s)),
        ("(",ws__(ExecLeft),")").map(|(_,e,_)|Arg::Command(e)),
        QuotedLitString.map(|s|Arg::StringLit(s)),
    )
}

parser! { (ArgP->Arg)
    or!(
        r_hash.map(|s|Arg::RawString(s) ) ,
        plus(StringPart).map(|v| match v.len(){
            1 => v[0].clone(),
            _=> Arg::StringExpr(v),
        }),
        ("\"",star(QuotedStringPart),"\"").map(|(_,v,_)| match v.len(){
            0=> Arg::StringLit(String::new()),
            1 => v[0].clone(),
            _=> Arg::StringExpr(v),
        }),
    )
}

/// Raw strings eg: r###" Any \ "##  wierd \ string "###
pub fn r_hash<'a>(it: &LCChars<'a>) -> ParseRes<'a, String> {
    let (it, (_, v, _), _) = ("r", "#".plus(), "\"").parse(it)?;
    Any.until(("\"", "#".exact(v.len())))
        .map(|(s, _)| s)
        .parse(&it)
}
