use crate::args::{Arg, Args};
use crate::channel::Channel;
use crate::exec::{Connection, Exec};
use crate::expr::Expr;
use crate::statement::Statement as Stt;
use bogobble::*;

char_bool! {RuSpecial,
    "=#&$|^{}()[]\\\" \n\t<>,;"
}

char_bool! {Letter,
    Any.except((RuSpecial,NumDigit))
}

char_bool! {LetterNum,
    Any.except(RuSpecial)
}

parser! {(Ident->String),
    or!(
        string((Letter.one(),LetterNum.istar())),
        ('"',string(Any.except('"').istar()),'"').map(|(_,b,_)|b)
    )

}

parser! {(ArgSpace->()),
    star(or_ig!(WS.iplus(),"\\\n",("#",Comment,"\n"))).ig()
}

parser! {(Path->String)
    string((maybe("~"),plus(or_ig!("\\ ",("/.",LetterNum).iplus()))))
}

parser! {(Builtin->&'static str)
    or!("cd","load","proglist","var","scope_depth")
}

parser! {(Assigner->&'static str)
    or!("let","set","export","push")
}

parser! {(End->())
    ws_(or_ig!("\n;".one(),EOI))
}

parser! {(Comment -> ())
    ("#",not("\n;").istar()).ig()
}

parser! {(Empties->())
    star(ws_(or_ig!(
            "\n;".plus(),
            Comment,
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
        (keyword(Assigner),plus(ws_(Ident)),ws_("="),ArgsS).map(|(mode,ids,_,args)|Stt::Assign(mode,ids,args)),
        (keyword("for"),plus_until(ws_(Ident),ws_(keyword("in"))),ArgsP,Block).map(|(_,(vars,_),args,block)|Stt::For{vars,args,block}),
        (keyword("if"),ws_(ExprRight),Block,maybe((wn_(keyword("else")),Block))).map(|(_,expr,block,op)|Stt::If{expr,block,else_:op.map(|(_,a)|a)}),
        (keyword("disown"),ws_(PExec)).map(|(_,e)|Stt::Disown(e)),
        (keyword(Builtin),ws_(ArgsS)).map(|(c,a)|Stt::Builtin(c,a)),
        (fail_on(keyword(or!("for","export","let","if","else","disown",Builtin))),
        ExprRight).map(|(_,e)|Stt::Expr(e)),
    )
}

parser! {(Block->Vec<Stt>)
    (wn_("{"),star_until(wn_(FullStatement),keyword("}"))).map(|(_,(a,_))|a)
}

parser! {(ExprLeft ->Expr)
    (PExec,ws_(maybe((ExChannel,ws_(">"),exists(">"),ws_(ArgP))))).map(|(exec,wop)|{
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

// At least one Arg
parser! {(ArgsP->Args)
    plus((ArgSpace,ArgP).last()).map(|v|Args(v))
}

// The list of args following a program
parser! {(ArgsS -> Args)
    star((ArgSpace,ArgP).last()).map(|v| Args(v))
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

parser! { (Var -> Arg)
    or!(
        ("${",sep_plus(ws__(Ident),"|"),maybe((",",ws__(ArgP))),"}").map(|(_,s,op,_)|Arg::VarList(s,op.map(|(_,b)|Box::new(b)))),
        ("$",Ident).map(|(_,s)|Arg::Var(s)),
    )
}

parser! {(StringPart->Arg)
    or!(
        ("$[",ws__(PExec),"]").map(|(_,e,_)|Arg::ArrCommand(e)),
        ("$(",ws__(PExec),")").map(|(_,e,_)|Arg::Command(e)),
        Var,
        LitString.map(|s|Arg::StringLit(s)),
    )
}

parser! {(QuotedStringPart->Arg)
    or!(
        ("$[",ws__(PExec),"]").map(|(_,e,_)|Arg::ArrCommand(e)),
        ("$(",ws__(PExec),")").map(|(_,e,_)|Arg::Command(e)),
        Var,
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
        ("[",ArgsS,"]").map(|(_,a,_)|Arg::List(a)),
        (ws__("{"),sep_until_ig((Ident,ws__("="),ArgP).map(|(k,_,v)|(k,v)),ws__(";"),ws__("}"))).map(|(_,s)|Arg::Map(s)),
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    pub fn arg_space() {
        let s = " \\\nfish";
        let r = (ArgSpace, "f").parse_s(s);
        assert_eq!(r, Ok(((), "f")));
    }
}
