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
    Path,
    Exec,
    Esc,
    Lit,
    Command,
    Var,
    Arg,
    Args,
    String,
    Quoted,
    Expr,
    End,
}

impl Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Item::Keyword => write!(f, "{}", color::Fg(color::Yellow)),
            Item::Statement => write!(f, "{}", color::Fg(color::LightMagenta)),
            Item::Symbol => write!(f, "{}", color::Fg(color::Blue)),
            Item::Ident | Item::Path => write!(f, "{}", color::Fg(color::Reset)),
            Item::String => write!(f, "{}", color::Fg(color::LightGreen)),
            Item::Lit => write!(f, "{}", color::Fg(color::LightYellow)),
            Item::Esc => write!(f, "{}", color::Fg(color::LightBlue)),
            //Item::Err => write!(f, "{}", color::Fg(color::Red)),
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

parser! {(ExprLeft ->PT)
    p_list!((Item::Expr) PExec,ws_(pmaybe(p_list!((Item::Command) ExChannel,sym(">"),pmaybe(sym(">"),Item::Symbol),ws_(ArgP)),Item::Command)))
}

parser! {(ExprRight -> PT)
    p_list!((Item::Expr) ExprLeft,pmaybe(ws_(sym(or("&&","||"))),Item::Symbol),wn_(ExprRight))
}

parser! {(ExTarget->PT)
     p_list!((Item::Exec) sym("|"),ws_(PExec))
}

parser! {(PConnection->PT)
    p_list!((Item::Exec) ExChannel,ExTarget)
}

parser! {(Path->PT)
    tpos((maybe("~"),p_plus(or_ig!("\\ ",("/._",Alpha,NumDigit).iplus()))),Item::Path)
}
parser! {(PExec->PT)
    p_list!((Item::Exec) tpos(Path,Item::Command) , Args,pmaybe(ws_(PConnection),Item::Exec))
}

parser! {(Args -> PT)
    vpos(p_star(ws_(ArgP)),Item::Args)
}

parser! { (QuotedLitString->PT)
    vpos(p_plus(or!(
            tpos(not("${}()[]\\\"").iplus(),Item::Lit),
            tpos(("\\",or_ig!(Any.one(),EOI)),Item::Esc),
    )),Item::Quoted)
}

parser! { (LitString->PT)
    vpos(p_plus(or!(
            tpos(not("&$|^{}()[]\\\" \n\t<>;").plus(),Item::String),
            tpos(("\\",or_ig!(Any.one(),EOI)),Item::Esc),
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

    Any.p_until(
        or!(sym(("\"", "#".exact(hlen))), tpos(EOI, Item::String)),
        Item::String,
    )
    .parse(&it2)
    .map_v(move |(q, h)| pt.clone().push(q).push(h))
}
