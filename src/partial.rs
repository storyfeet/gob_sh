use bogobble::partial::*;
use bogobble::*;
use std::fmt::{self, Display, Write};
use termion::color;
use transliterate::parser::*;
use transliterate::*;

//type PT = PosTree<Item>;

pub struct PConfig {}

impl PConfig {
    fn put_item(&self, i: Item, s: &mut String) {
        //TODO Enable use of RU_HIGHLIGHT var
        write!(s, "{}", i).ok();
    }
}

impl SSParser<PConfig> for Item {
    fn ss_parse<'a>(&self, it: &PIter<'a>, res: &mut String, c: &PConfig) -> SSRes<'a> {
        c.put_item(*self, res);
        Ok((it.clone(), None))
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
    fn ss_parse<'a>(&self, it: &PIter<'a>, res: &mut String, cf: &PConfig) -> SSRes<'a> {
        (self.item, BRP(&self.p), WS.istar()).ss_parse(it, res, cf)
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

ss_parser! {End,
    WS_(ss_or!("\n;".one(),EOI))
}

ss_parser! {(Empties,PConfig),
    PStar(WS_(ss_or!(
            "\n;".iplus(),
            ("#",not("\n;").istar()),
        )))
}

ss_parser! {(ExChannel,PConfig),
    sym(ss_or!( "^^", "^", ""))
}

ss_parser! {(Lines,PConfig),
    (Empties, PStar(WS_(FullStatement)),EOI),
}

ss_parser! {(FullStatement,PConfig),
    (Statement,PPlus(End),Empties)
}

ss_parser! {(Id,PConfig),
    WS__(common::Ident)
}

ss_parser! {(Idents,PConfig),
    PPlus(WS__(common::Ident)),
}

ss_parser! { (Statement,PConfig),
    ss_or!(
        (kw("let"), Idents,sym(WS_("=")),ArgsS),
        (kw("export"), Idents,sym(WS_("=")),ArgsS),
        (kw("cd"),WS_(ArgsS)),
        (kw("for"),WS_(ArgsS),PlusUntil(Id,kw("in")),ArgsP,Block),
        (kw("if"),WS_(ExprRight),Block,Maybe((WN_(kw("else")),Block))),
        (kw("disown"),PExec),
        (sym(". "),WS_(Path)),
    )
}

/*parser! {(Statement->PT)
    or!(
        p_list!((Item::Statement) kw("let"),Idents,sym(ws_("=")),ArgsS),
        p_list!((Item::Statement) kw("export"),Idents,sym(ws_("=")),ArgsS),
        p_list!((Item::Statement) kw("cd"),ws_(ArgP)),
        p_list!((Item::Statement) kw("for"),vpos(plus_until(Id,or(kw("in"),sym(EOI))).map(|(mut v,e)|{v.push(e);v}),Item::Ident),ArgsP,Block),
        p_list!((Item::Statement) kw("if"),ws_(ExprRight),Block,Maybe(p_list!((Item::Statement) WN_(kw("else")),Block),Item::Statement)),
        p_list!((Item::Statement) kw("disown"),PExec),
        p_list!((Item::Statement) sym(". "),ws_(Path)),
        ExprRight,
    )
}*/

ss_parser! {(Block,PConfig),
    (WN_(sym("{")),PPlus(WN_(FullStatement)),WN_(sym("}")))
    //p_list!((Item::Statement) WN_(sym("{")),vpos(p_plus(WN_(FullStatement)),Item::Statement),WN_(sym("}"))),
}

ss_parser! {(ExprLeft ,PConfig),
    (PExec,WS_(Maybe(sym((">",Maybe(">"))))),WS_(ArgP))
    //p_list!((Item::Expr) PExec,ws_(pMaybe(p_list!((Item::Command) ExChannel,sym(">"),Maybe(sym(">"),Item::Symbol),ws_(ArgP)),Item::Command)))
}

ss_parser! {(ExprRight,PConfig),
    (ExprLeft,Maybe((WS_(sym(ss_or!("&&","||"))),WN_(ExprRight))))
    /*p_list!(
        (Item::Expr)
        ExprLeft,
        Maybe(ws_(sym(or("&&","||"))).merge(Item::Expr,WN_(ExprRight)),Item::Expr),
    )*/
}

ss_parser! {(ExTarget,PConfig),
    (sym("|"),WS_(PExec))
     //p_list!((Item::Exec) sym("|"),ws_(PExec))
}

ss_parser! {(PConnection,PConfig),
    (ExChannel,ExTarget)
}

ss_parser! {Path,
    ((Maybe("~"),PPlus(ss_or!("\\ ",ss(("/._-",Alpha,NumDigit).iplus())))))
}

ss_parser! {(PExec,PConfig),
    (Path,ArgsS,Maybe(WS_(PConnection)))
    //tpos(Path,Item::Command).merge(Item::Command,ArgsS).opush(Maybe(ws_(PConnection)))
}

ss_parser! {(ArgsS ,PConfig),
    PStar(WS_(ArgP))
}
ss_parser! {(ArgsP ,PConfig),
    PPlus(WS_(ArgP))
}

ss_parser! { QuotedLitString,
    PPlus(ss_or!(
            ss(not("${}()[]\\\"").iplus()),
            ("\\",ss_or!(Any.one(),EOI)),
    ))
}

ss_parser! { (LitString,PConfig),
   PPlus(ss_or!(
            not("#&$|^{}()[]\\\" \n\t<>;").iplus(),
            ("\\",ss_or!(ss(Any.one()),EOI)),
    ))
}

ss_parser! {(StringPart,PConfig),
    ss_or!(
        (Put(Item::Command), sym("$["),WS__(PExec),sym("]")),
        (Put(Item::Command), sym("$("),WS__(PExec),sym(")")),
        (Put(Item::Var), sym("${"),WS__(ss((Alpha,NumDigit,"_").iplus())),sym("}")),
        (Put(Item::Var), sym("$"),(Alpha,NumDigit,"_").iplus()),
        LitString,
    )
}

ss_parser! {(QuotedStringPart,PConfig),
    ss_or!(
        (Put(Item::Command) ,sym("$["),WS__(PExec),sym("]")),
        (Put(Item::Command), sym("$("),WS__(PExec),sym(")")),
        (Put(Item::Var), sym("${"),WS__((Alpha,NumDigit,"_").iplus()),sym("}")),
        (Put(Item::Var),sym("$"),(Alpha,NumDigit,"_").iplus()),
        QuotedLitString,
    )
}

ss_parser! {(ArgP,PConfig),
    ss_or!(
        PRHash,
        PPlus(StringPart),
        (Put(Item::String),sym("\""),Put(Item::Quoted),PStar(QuotedStringPart),sym("\""))
    )
}

/// partial Raw strings eg: r###" Any \ "##  wierd \ string "###
pub struct PRHash;

impl SSParser<PConfig> for PRHash {
    fn ss_parse<'a>(&self, it: &PIter<'a>, res: &mut String, cf: &PConfig) -> SSRes<'a> {
        let mut i2 = it.clone();
        match i2.next() {
            Some('r') => {}
            None => {
                cf.put_item(Item::Symbol, res);
                res.push_str(it.str_to(None));
                return Ok((i2, None));
            }
            _ => return it.err_r(Expected::Str("RawString")),
        }

        //count hashes
        let mut nhashes = 0;
        loop {
            match i2.next() {
                Some('#') => nhashes += 1,
                Some('"') => {
                    cf.put_item(Item::Symbol, res);
                    res.push_str(it.str_to(i2.index()));
                    break;
                }
                Some(_) => return Err(i2.err_s("RawString")),
                None => return Ok((i2, None)),
            }
        }
        cf.put_item(Item::Quoted, res);
        let raw_start = i2.clone();
        let mut raw_fin = i2.clone();
        loop {
            match i2.next() {
                Some('"') => {
                    let mut i3 = it.clone();
                    for _ in 0..nhashes {
                        match i3.next() {
                            Some('#') => {}
                            _ => {
                                raw_fin = i2.clone();
                                continue;
                            }
                        }
                    }
                    res.push_str(raw_start.str_to(raw_fin.index()));
                    cf.put_item(Item::Symbol, res);
                    res.push_str(raw_fin.str_to(i3.index()));
                    return Ok((i3, None));
                }
                Some(_) => raw_fin = i2.clone(),

                None => {
                    res.push_str(raw_start.str_to(None));
                    return Ok((i2, None));
                }
            }
        }
    }
}
