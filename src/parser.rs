use crate::args::Arg;
use crate::channel::Channel;
use crate::exec::{Connection, Exec};
use crate::statement::Statement as Stt;
use bogobble::*;

parser! {(End->())
    ws_(or_ig!("\n;".one(),EOI))
}

parser! {(ExChannel ->Channel)
    or!(
        "^^".asv(Channel::Both),
        "^".asv(Channel::StdErr),
        "".asv(Channel::StdOut),

    )
}

parser! {(FullStatement->Stt)
    first(Statement,End)
}

parser! {(Statement->Stt)
    or!(
        (keyword("let"),plus(ws_(common::Ident)),ws_("="),Args).map(|(_,ids,_,args)|Stt::Let(ids,args)),
        (PExec,ws_(maybe((ExChannel,">",exists(">"),ws_(ArgP))))).map(|(exec,wop)|{
            match wop {
                Some((chan,_,append,filename))=>Stt::Write{exec,chan,append,filename},
                None=>Stt::Exec(exec),
            }})
    )
}

parser! {(ExTarget->Exec)
     ("|",ws_(PExec)).map(|(_,e)|e),
}

parser! {(PConnection->Connection)
    (ExChannel,ExTarget).map(|(chan,target)|Connection{chan,target:Box::new(target)})
}

parser! {(PExec->Exec)
    (common::Ident , Args,maybe(ws_(PConnection))).map(|(command,args,conn)|Exec{command,args,conn})
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
            not("$|^{}()[]\\\" \n\t<>").plus(),
            "\\n".map(|_|"\n".to_string()),
            "\\t".map(|_|"\t".to_string()),
             ("\\",Any.one()).map(|(_,c)| {let mut s=String::new(); s.push(c);s}),
    ))
}

parser! {(StringPart->Arg)
    or!(
        ("$[",ws__(PExec),"]").map(|(_,e,_)|Arg::ArrCommand(e)),
        ("$(",ws__(PExec),")").map(|(_,e,_)|Arg::Command(e)),
        ("${",ws__((Alpha,NumDigit,"_").plus()),"}").map(|(_,s,_)|Arg::Var(s)),
        ("$",(Alpha,NumDigit,"_").plus()).map(|(_,s)|Arg::Var(s)),
        LitString.map(|s|Arg::StringLit(s)),
    )
}

parser! {(QuotedStringPart->Arg)
    or!(
        ("$",(Alpha,NumDigit,"_").plus()).map(|(_,s)|Arg::Var(s.to_string())),
        ("${",ws__((Alpha,NumDigit,"_").plus()),"}").map(|(_,s,_)|Arg::Var(s.to_string())),
        ("(",ws__(PExec),")").map(|(_,e,_)|Arg::Command(e)),
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
pub fn r_hash<'a>(it: &PIter<'a>) -> ParseRes<'a, String> {
    let (it, (_, v, _), _) = ("r", "#".star(), "\"").parse(it)?;
    Any.until(("\"", "#".exact(v.len())))
        .map(|(s, _)| s.to_string())
        .parse(&it)
}
