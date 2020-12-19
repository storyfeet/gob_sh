use crate::parser::r_hash;
use bogobble::partial::*;
use bogobble::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Keyword,
    Statement,
    Symbol,
    Redirect,
    Ident,
    Exec,
    Lit,
    Command,
    Arg,
    Err,
}

fn kw<'a, P: Parser<'a>>(p: P) -> PosTreeParse<KeyWord<P>, Item> {
    PosTreeParse {
        p: keyword(p),
        item: Item::Keyword,
    }
}

fn sym<'a, P: Parser<'a>>(p: P) -> PosTreeParse<P, Item> {
    PosTreeParse {
        p,
        item: Item::Symbol,
    }
}

parser! {(End->())
    ws_(or_ig!("\n;".one(),EOI))
}

parser! {(ExChannel -> PosTree<Item>)
    sym(or!( "^^", "^", ""))
}

parser! {(FullStatement->PosTree<Item>)
    //TODO
    tpos(first(Statement,End).asv(true),Item::Statement)
}

parser! {(Statement->PosTree<Item>)
    or!(
        p_list!((Item::Statement) kw("let"),tpos(plus(ws_(common::Ident)),Item::Ident),sym(ws_("=")),Args),
        p_list!((Item::Statement) PExec,ws_(pmaybe(
                    p_list!((Item::Redirect) ExChannel,sym(or!(">>",">")),ws_(ArgP)),
                    Item::Redirect,
                )))
    )
}

parser! {(ExTarget->PosTree<Item>)
     p_list!((Item::Exec) sym("|"),ws_(PExec))
}

parser! {(PConnection->PosTree<Item>)
    p_list!((Item::Exec) ExChannel,ExTarget)
}

parser! {(PExec->PosTree<Item>)
    p_list!((Item::Exec) tpos(common::Ident,Item::Command) , Args,pmaybe(ws_(PConnection),Item::Exec))
}

parser! {(Args -> PosTree<Item>)
    //TODO add star and plus to partials
    tpos(star(ws_(ArgP))
}

parser! { (QuotedLitString->String)
    strings_plus(or!(
            string(not("${}()[]\\\"").plus()),
            "\\n".map(|_|"\n".to_string()),
            "\\t".map(|_|"\t".to_string()),
             ("\\",Any.one()).map(|(_,c)| {let mut s=String::new(); s.push(c);s}),
    ))
}

parser! { (LitString->String)
    strings_plus(or!(
            string(not("$|^{}()[]\\\" \n\t<>").plus()),
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
        ("$",(Alpha,NumDigit,"_").plus()).map(|(_,s)|Arg::Var(s.to_string())),
        ("${",ws__(string((Alpha,NumDigit,"_").plus())),"}").map(|(_,s,_)|Arg::Var(s.to_string())),
        ("(",ws__(PExec),")").map(|(_,e,_)|Arg::Command(e)),
        QuotedLitString.map(|s|Arg::StringLit(s)),
    )
}

parser! { (ArgP->PosTree<Item>)
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
