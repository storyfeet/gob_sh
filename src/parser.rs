use crate::args::Arg;
use crate::channel::Channel;
use crate::exec::{Connection, Exec};
use crate::statement::Statement as Stt;
use bogobble::*;

parser! {(Path->String)
    string((maybe("~"),plus(or_ig!("\\ ",("/._",Alpha,NumDigit).iplus()))))
}

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

parser! {(Lines -> Vec<Stt>)
    first(star(ws_(FullStatement)),EOI)
}

parser! {(FullStatement->Stt)
    first(Statement,bogobble::partial::p_plus(End))
}

parser! {(Statement->Stt)
    or!(
        (keyword("let"),plus(ws_(common::Ident)),ws_("="),Args).map(|(_,ids,_,args)|Stt::Let(ids,args)),
        (keyword("export"),plus(ws_(common::Ident)),ws_("="),Args).map(|(_,ids,_,args)|Stt::Export(ids,args)),
        (keyword("cd"),ws_(ArgP)).map(|(_,a)|Stt::Cd(a)),
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
    (Path , Args,maybe(ws_(PConnection))).map(|(command,args,conn)|Exec{command,args,conn})
}

parser! {(Args -> crate::args::Args)
    star(ws_(ArgP)).map(|v| crate::args::Args(v))
}

parser! { (QuotedLitString->String)
    strings_plus(or!(
            string(not("${}()[]\\\"").plus()),
            "\\n".map(|_|"\n".to_string()),
            "\\t".map(|_|"\t".to_string()),
            "\\e".map(|_|"\u{1b}[".to_string()),
             ("\\",Any.one()).map(|(_,c)| {let mut s=String::new(); s.push(c);s}),
    ))
}

parser! { (LitString->String)
    strings_plus(or!(
            string(not("$|^{}()[]\\\" \n\t<>;").plus()),
            "\\n".map(|_|"\n".to_string()),
            "\\t".map(|_|"\t".to_string()),
             ("\\",Any.one()).map(|(_,c)| {let mut s=String::new(); s.push(c);s}),
    ))
}

parser! {(StringPart->Arg)
    or!(
        ("$[",ws__(PExec),"]").map(|(_,e,_)|Arg::ArrCommand(e)),
        ("$(",ws__(PExec),")").map(|(_,e,_)|Arg::Command(e)),
        ("${",ws__(string((Alpha,NumDigit,"_").plus())),"}").map(|(_,s,_)|Arg::Var(s)),
        ("$",string((Alpha,NumDigit,"_").plus())).map(|(_,s)|Arg::Var(s)),
        LitString.map(|s|Arg::StringLit(s)),
    )
}

parser! {(QuotedStringPart->Arg)
    or!(
        ("$[",ws__(PExec),"]").map(|(_,e,_)|Arg::ArrCommand(e)),
        ("$(",ws__(PExec),")").map(|(_,e,_)|Arg::Command(e)),
        ("${",ws__(string((Alpha,NumDigit,"_").plus())),"}").map(|(_,s,_)|Arg::Var(s)),
        ("$",string((Alpha,NumDigit,"_").plus())).map(|(_,s)|Arg::Var(s)),
        QuotedLitString.map(|s|Arg::StringLit(s)),
    )
}

parser! {(QuotedString->Arg)
    star(QuotedStringPart).map(|v| match v.len(){
            0=> Arg::StringLit(String::new()),
            1 => v[0].clone(),
            _=> Arg::StringExpr(v),
        }),

}

parser! { (ArgP->Arg)
    or!(
        ("~",keyword(maybe(LitString))).map(|(_,s)|Arg::HomePath(s.unwrap_or(String::new()))),
        ("~",plus(StringPart)).map(|(_,v)| Arg::HomeExpr(v)),
        r_hash.map(|s|Arg::RawString(s) ) ,
        plus(StringPart).map(|v| match v.len(){
            1 => v[0].clone(),
            _=> Arg::StringExpr(v),
        }),
        ("\"",QuotedString,"\"").map(|(_,s,_)|s),
        )
}

/// Raw strings eg: r###" Any \ "##  wierd \ string "###
pub fn r_hash<'a>(it: &PIter<'a>) -> ParseRes<'a, String> {
    let (it, (_, v, _), _) = ("r", "#".star(), "\"").parse(it)?;
    Any.until(("\"", "#".exact(v.len())))
        .map(|(s, _)| s.to_string())
        .parse(&it)
}
