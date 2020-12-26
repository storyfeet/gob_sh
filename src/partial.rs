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
        p_list!((Item::Command) sym("$"),tpos((Alpha,NumDigit,"_").plus(),Item::Command)),
        p_list!((Item::Var) sym("${"),ws__(tpos((Alpha,NumDigit,"_").plus(),Item::Var)),sym("}")),
        p_list!((Item::Command) sym("("),ws__(PExec),sym(")")),
        QuotedLitString,
    )
}

parser! { (ArgP->PT)
    or!(
        p_r_hash,
        vpos(plus(StringPart),Item::Arg),
        ("\"",star(QuotedStringPart),"\"").map(|(_,v,_)| match v.len(){
            0=> Arg::StringLit(String::new()),
            1 => v[0].clone(),
            _=> Arg::StringExpr(v),
        }),
    )
}

/// partial Raw strings eg: r###" Any \ "##  wierd \ string "###
pub fn p_r_hash<'a>(it: &PIter<'a>) -> ParseRes<'a, PT> {
    let (it2, pt, e) = p_list!((Item::String) sym("r"), sym("#".star()), sym("\"")).parse(it)?;
    let hlen = match pt.children.get(1) {
        Some(ch) => ch.str_len(it.orig_str()),
        None => return EOI.parse(&it2).map_v(|_| pt).map_err(|e2| e.unwrap_or(e2)),
    };

    Any.until(sym(("\"", "#".exact(hlen))))
        .parse(&it2)
        .map_v(|(q, h)| pt.push(PosTree::new().push(h)))
    }
}
