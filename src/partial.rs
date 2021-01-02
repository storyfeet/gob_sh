use bogobble::partial::*;
use bogobble::*;
use std::fmt::{self, Display};
use termion::color;

type PT = PosTree<Item>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Keyword,
    Statement,
    Symbol,
    Redirect,
    Ident,
    Exec,
    //Lit,
    Command,
    Var,
    Arg,
    Args,
    String,
    Quoted,
    End,
}

impl Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Item::Keyword => write!(f, "{}", color::Fg(color::Yellow)),
            Item::Statement => write!(f, "{}", color::Fg(color::LightMagenta)),
            Item::Symbol => write!(f, "{}", color::Fg(color::Blue)),
            Item::Ident => write!(f, "{}", color::Fg(color::White)),
            Item::String => write!(f, "{}", color::Fg(color::LightGreen)),
            //        Item::Err => write!(f, "{}", color::Fg(color::Yellow)),
            _ => write!(f, "{}", color::Fg(color::Reset)),
        }
    }
}

fn kw<'a, P: Parser<'a>>(p: P) -> PosTreeParse<KeyWord<P>, Item> {
    tpos(keyword(p), Item::Keyword)
}

fn sym<'a, P: Parser<'a>>(p: P) -> PosTreeParse<P, Item> {
    tpos(p, Item::Symbol)
}

parser! {(End->PT)
    tpos( ws_(or_ig!("\n;".one(),EOI)),Item::End)
}

parser! {(ExChannel -> PT)
    sym(or!( "^^", "^", ""))
}

parser! {(Lines->PT)
    vpos(first(p_star(ws_(FullStatement)),EOI),Item::Statement)
}

parser! {(FullStatement->PT)
    //TODO
    p_list!((Item::Statement) Statement,tpos(p_plus(End),Item::End))
}

parser! {(Statement->PT)
    or!(
        p_list!((Item::Statement) kw("let"),tpos(p_plus(ws_(common::Ident)),Item::Ident),sym(ws_("=")),Args),
        p_list!((Item::Statement) kw("export"),tpos(p_plus(ws_(common::Ident)),Item::Ident),sym(ws_("=")),Args),
        p_list!((Item::Statement) kw("cd"),ws_(ArgP)),
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
    vpos(p_star(ws_(ArgP)),Item::Args)
}

parser! { (QuotedLitString->PT)
    tpos(p_plus(or_ig!(
            not("${}()[]\\\"").iplus(),
             ("\\",Any.one()),
    )),Item::Quoted)
}

parser! { (LitString->PT)
    tpos(p_plus(or_ig!(
            not("$|^{}()[]\\\" \n\t<>;").plus(),
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
        p_list!((Item::Command) sym("$["),ws__(PExec),sym("]")),
        p_list!((Item::Command) sym("$("),ws__(PExec),sym(")")),
        p_list!((Item::Var) sym("${"),ws__(tpos((Alpha,NumDigit,"_").plus(),Item::Var)),sym("}")),
        p_list!((Item::Var) sym("$"),tpos((Alpha,NumDigit,"_").plus(),Item::Var)),
        QuotedLitString,
    )
}

parser! { (ArgP->PT)
    or!(
        p_r_hash,
        vpos(p_plus(StringPart),Item::Arg),
        p_list!((Item::String)sym("\""),vpos(p_star(QuotedStringPart),Item::String),sym("\""))
    )
}

/// partial Raw strings eg: r###" Any \ "##  wierd \ string "###
pub fn p_r_hash<'a>(it: &PIter<'a>) -> ParseRes<'a, PT> {
    let (it2, pt, e) = p_list!((Item::String) sym("r"), sym("#".star()), sym("\"")).parse(it)?;
    let hlen = match pt.children.get(1) {
        Some(ch) => ch.str_len(it.orig_str()),
        None => {
            return EOI
                .parse(&it2)
                .map_v(|_| pt.clone())
                .map_err(|e2| e.unwrap_or(e2))
        }
    };

    Any.p_until(sym(("\"", "#".exact(hlen))), Item::String)
        .parse(&it2)
        .map_v(move |(q, h)| pt.clone().push(q).push(h))
}
