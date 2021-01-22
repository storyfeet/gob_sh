use crate::args::{Arg, Args};
use crate::channel::Channel;
use crate::exec::{Connection, Exec};
use crate::expr::Expr;
use crate::statement::Statement as Stt;
use bogobble::*;

parser! {(Path->String)
    string((maybe("~"),plus(or_ig!("\\ ",("/._",Alpha,NumDigit).iplus()))))
}

parser! {(End->())
    ws_(or_ig!("\n;".one(),EOI))
}

parser! {(Empties->())
    star(ws_(or_ig!(
            "\n;".plus(),
            ("#",not("\n;").star()),
        ))).ig()
}

parser! {(ExChannel ->Channel)
    or!(
        "^^".asv(Channel::Both),
        "^".asv(Channel::StdErr),
        "".asv(Channel::StdOut),
    )
}

parser! {(Lines -> Vec<Stt>)
    (Empties,star(ws_(FullStatement)),EOI).map(|(_,a,_)|a)
}

parser! {(FullStatement->Stt)
    first(Statement,(End,Empties))
}

parser! {(Statement->Stt)
    or!(
        (keyword("let"),plus(ws_(common::Ident)),ws_("="),ArgsS).map(|(_,ids,_,args)|Stt::Let(ids,args)),
        (keyword("export"),plus(ws_(common::Ident)),ws_("="),ArgsS).map(|(_,ids,_,args)|Stt::Export(ids,args)),
        (keyword("cd"),ws_(ArgP)).map(|(_,a)|Stt::Cd(a)),
        (keyword("for"),plus_until(ws_(common::Ident),ws_(keyword("in"))),ArgsP,Block).map(|(_,(vars,_),args,block)|Stt::For{vars,args,block}),
        (keyword("if"),ws_(ExprRight),Block,maybe((wn_(keyword("else")),Block))).map(|(_,expr,block,op)|Stt::If{expr,block,else_:op.map(|(_,a)|a)}),
        (keyword("disown"),ws_(PExec)).map(|(_,e)|Stt::Disown(e)),
        (". ",ws_(Path)).map(|(_,p)|Stt::Dot(p)),
        (fail_on(keyword(or!("for","export","cd","let","if","else","disown"))),
        ExprRight).map(|(_,e)|Stt::Expr(e)),
    )
}

parser! {(Block->Vec<Stt>)
    (wn_("{"),star_until(wn_(FullStatement),keyword("}"))).map(|(_,(a,_))|a)
}

parser! {(ExprLeft ->Expr)
    (PExec,ws_(maybe((ExChannel,">",exists(">"),ws_(ArgP))))).map(|(exec,wop)|{
        match wop {
            Some((chan,_,append,filename))=>Expr::Write{exec,chan,append,filename},
            None=>Expr::Exec(exec),
        }})

}
parser! {(ExprRight -> Expr)
    (ExprLeft,maybe((ws_(or("&&","||")),wn_(ExprRight)))).map(|(lt,op)|{
        match op {
            Some(("&&",rt))=>Expr::And(Box::new(lt),Box::new(rt)),
            Some(("||",rt))=>Expr::Or(Box::new(lt),Box::new(rt)),
            _=>lt,
        }

    })
}

parser! {(ExTarget->Exec)
     ("|",ws_(PExec)).map(|(_,e)|e),
}

parser! {(PConnection->Connection)
    (ExChannel,ExTarget).map(|(chan,target)|Connection{chan,target:Box::new(target)})
}

parser! {(PExec->Exec)
    (Path , ArgsS,maybe(ws_(PConnection))).map(|(command,args,conn)|Exec{command,args,conn})
}

parser! {(ArgsP->Args)
    plus(ws_(ArgP)).map(|v|Args(v))
}

parser! {(ArgsS -> Args)
    star(ws_(ArgP)).map(|v| Args(v))
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
            string(not("#&$|^{}()[]\\\" \n\t<>;").plus()),
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
