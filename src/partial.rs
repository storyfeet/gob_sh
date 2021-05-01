use bogobble::partial::*;
use bogobble::*;
use std::fmt::{self, Display, Write};
use termion::color;
#[macro_use]
use transliterate::*;
use transliterate::parser::*;

type PT = PosTree<Item>;

struct PConfig {}

impl PConfig {
    fn item_str(&self, i: Item, s: &mut String) {
        //TODO Enable use of RU_HIGHLIGHT
        write!(s, "{}", i).ok();
    }
}

impl SSParser<PConfig> for Item {
    fn ss_parse<'a>(&self, it: &PIter<'a>, res: &mut String, c: &PConfig) -> ParseRes<'a, ()> {
        c.item_str(*self, res);
        Ok((it.clone(), (), None))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Keyword,
    Statement,
    Symbol,
    Ident,
    Path,
    Exec,
    Esc,
    Lit,
    Command,
    Comment,
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
            Item::Comment => write!(f, "{}", color::Fg(color::LightBlue)),
            _ => write!(f, "{}", color::Fg(color::Reset)),
        }
    }
}

pub struct ItemWrap<P: SSParser<PConfig>> {
    p: P,
    item: Item,
}

impl<P: SSParser<PConfig>> SSParser<PConfig> for ItemWrap<P> {
    //TODO allow partials
    fn ss_parse<'a>(&self, it: &PIter<'a>, res: &mut String, cf: &PConfig) -> ParseRes<'a, ()> {
        (self.item, self.p, ss(WS.star())).ss_parse(it, res, cf)
    }
}

fn kw<P: SSParser<PConfig>>(p: P) -> ItemWrap<KeyWord<P>> {
    ItemWrap {
        p: KeyWord(p),
        item: Item::Keyword,
    }
}

fn sym<P: SSParser<PConfig>>(p: P) -> ItemWrap<KeyWord<P>> {
    ItemWrap {
        p: KeyWord(p),
        item: Item::Symbol,
    }
}

parser! {(End->PT)
    tpos( ws_(or_ig!("\n;".one(),EOI)),Item::End)
}

parser! {(Empties->PT)
    tpos(star(ws_(or_ig!(
            "\n;".plus(),
            ("#",not("\n;").star()),
        ))),Item::Comment)
}

ss_parser! {ExChannel,
    sym(ss_or!( "^^", "^", ""))
}

parser! {(Lines->PT)
    p_list!((Item::Statement)Empties, vpos(first(p_star(ws_(FullStatement)),EOI),Item::Statement))
}

parser! {(FullStatement->PT)
    p_list!((Item::Statement) Statement,tpos(p_plus(End),Item::End),Empties)
}

parser! {(Id->PT)
    ws__(tpos(common::Ident,Item::Ident))
}

parser! {(Idents->PT)
    tpos(p_plus(ws__(common::Ident)),Item::Ident)
}

ss_parser! { Statement,
    ss_or!(
        (kw("let"), Idents,sym(ws_("=")),ArgsS),
        (kw("export"), Idents,sym(ws_("=")),ArgsS),
        (kw("cd"),ws_(ArgS)),
        (kw("for"),ws_(ArgS),plus_until(Id,(kw("in")),ArgsP,Block),
        (kw("if"),ws_(ExprRight),Block,maybe(wn_(kw("else")),Block)),
    )
}

/*parser! {(Statement->PT)
    or!(
        p_list!((Item::Statement) kw("let"),Idents,sym(ws_("=")),ArgsS),
        p_list!((Item::Statement) kw("export"),Idents,sym(ws_("=")),ArgsS),
        p_list!((Item::Statement) kw("cd"),ws_(ArgP)),
        p_list!((Item::Statement) kw("for"),vpos(plus_until(Id,or(kw("in"),sym(EOI))).map(|(mut v,e)|{v.push(e);v}),Item::Ident),ArgsP,Block),
        p_list!((Item::Statement) kw("if"),ws_(ExprRight),Block,pmaybe(p_list!((Item::Statement) wn_(kw("else")),Block),Item::Statement)),
        p_list!((Item::Statement) kw("disown"),PExec),
        p_list!((Item::Statement) sym(". "),ws_(Path)),
        ExprRight,
    )
}*/

ss_parser! {Block,
    (wn_(sym("{")),vpos(p_plus(wn_(FullStatement)),wn_(sym("}"))))
    //p_list!((Item::Statement) wn_(sym("{")),vpos(p_plus(wn_(FullStatement)),Item::Statement),wn_(sym("}"))),
}

ss_parser! {ExprLeft ,
    (PExec,ws_(pmaybe(ExChannel,sym((">",maybe(">")))),ws_(ArgP)))
    //p_list!((Item::Expr) PExec,ws_(pmaybe(p_list!((Item::Command) ExChannel,sym(">"),pmaybe(sym(">"),Item::Symbol),ws_(ArgP)),Item::Command)))
}

ss_parser! {ExprRight,
    (ExprLeft,maybe((ws_(sym(ss_or!("&&","||"))),wn_(ExprRight))))
    /*p_list!(
        (Item::Expr)
        ExprLeft,
        pmaybe(ws_(sym(or("&&","||"))).merge(Item::Expr,wn_(ExprRight)),Item::Expr),
    )*/
}

parser! {(ExTarget->PT)
     p_list!((Item::Exec) sym("|"),ws_(PExec))
}

parser! {(PConnection->PT)
    p_list!((Item::Exec) ExChannel,ExTarget)
}

parser! {(Path->PT)
    tpos((maybe("~"),p_plus(or_ig!("\\ ",("/._-",Alpha,NumDigit).iplus()))),Item::Path)
}
parser! {(PExec->PT)
    tpos(Path,Item::Command).merge(Item::Command,ArgsS).opush(maybe(ws_(PConnection)))
}

parser! {(ArgsS -> PT)
    vpos(p_star(ws_(ArgP)),Item::Args)
}
parser! {(ArgsP -> PT)
    vpos(p_plus(ws_(ArgP)),Item::Args)
}

parser! { (QuotedLitString->PT)
    vpos(p_plus(or!(
            tpos(not("${}()[]\\\"").iplus(),Item::Lit),
            tpos(("\\",or_ig!(Any.one(),EOI)),Item::Esc),
    )),Item::Quoted)
}

parser! { (LitString->PT)
    vpos(p_plus(or!(
            tpos(not("#&$|^{}()[]\\\" \n\t<>;").plus(),Item::String),
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
    let hlen = match (pt.children.get(1), pt.children.get(2)) {
        (Some(ch), Some(_)) => ch.str_len(it.orig_str()),
        (Some(ch), None) if ch.on_str(it.orig_str()) == "\"" => 0,
        (Some(_), None) => {
            return EOI
                .parse(&it2)
                .map_v(|_| pt.clone())
                .map_err(|e2| e.unwrap_or(e2))
        }
        _ => return Err(it.err_s("Quotes")),
    };

    Any.p_until(
        or!(sym(("\"", "#".exact(hlen))), tpos(EOI, Item::String)),
        Item::String,
    )
    .parse(&it2)
    .map_v(move |(q, h)| pt.clone().push(q).push(h))
}
