use crate::parser::r_hash;
use bogobble::partial::*;
use bogobble::*;

type PT = PosTree<Item>;

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
    Var,
    Arg,
    Args,
    String,
    Quoted,
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

parser! {(ExChannel -> PT)
    sym(or!( "^^", "^", ""))
}

parser! {(FullStatement->PT)
    //TODO
    tpos(first(Statement,End).asv(true),Item::Statement)
}

parser! {(Statement->PT)
    or!(
        p_list!((Item::Statement) kw("let"),tpos(plus(ws_(common::Ident)),Item::Ident),sym(ws_("=")),Args),
        p_list!((Item::Statement) PExec,ws_(pmaybe(
                    p_list!((Item::Redirect) ExChannel,sym(or!(">>",">")),ws_(ArgP)),
                    Item::Redirect,
                )))
    )
}

parser! {(ExTarget->PT)
     p_list!((Item::Exec) sym("|"),ws_(PExec))
}

parser! {(PConnection->PT)
    p_list!((Item::Exec) ExChannel,ExTarget)
}

parser! {(PExec->PT)
    p_list!((Item::Exec) tpos(common::Ident,Item::Command) , Args,pmaybe(ws_(PConnection),Item::Exec))
}

parser! {(Args -> PT)
    vpos(star(ws_(ArgP)),Item::Args)
}

parser! { (QuotedLitString->PT)
    tpos(plus(or_ig!(
            not("${}()[]\\\"").iplus(),
             ("\\",Any.one()),
    )),Item::Quoted)
}

parser! { (LitString->PT)
    tpos(plus(or_ig!(
            not("$|^{}()[]\\\" \n\t<>").plus(),
             ("\\",Any.one()),
    )),Item::String)
}

parser! {(StringPart->PT)
    or!(
        p_list!((Item::Command) sym("$["),ws__(PExec),sym("]")),
        p_list!((Item::Command) sym("$("),ws__(PExec),sym(")")),
        p_list!((Item::Var) sym("${"),ws__(tpos((Alpha,NumDigit,"_").plus(),Item::Var)),sym("}")),
        p_list!((Item::Var) sym("$"),tpos((Alpha,NumDigit,"_").plus(),Item::Var)),
        LitString,
    )
}

parser! {(QuotedStringPart->PT)
    or!(
        p_list!((Item::Command) (sym("$"),tpos(Alpha,NumDigit,"_").plus()).map(|(_,s)|Arg::Var(s.to_string())),
        ("${",ws__(string((Alpha,NumDigit,"_").plus())),"}").map(|(_,s,_)|Arg::Var(s.to_string())),
        ("(",ws__(PExec),")").map(|(_,e,_)|Arg::Command(e)),
        QuotedLitString.map(|s|Arg::StringLit(s)),
    )
}

parser! { (ArgP->PT)
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
