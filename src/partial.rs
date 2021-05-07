use bogobble::partial::*;
use bogobble::*;
use std::fmt::{self, Debug, Display, Write};
use termion::color;
use transliterate::bo_part::*;
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
            Item::Var => write!(f, "{}", color::Fg(color::LightMagenta)),
            Item::Ident | Item::Path => write!(f, "{}", color::Fg(color::Reset)),
            Item::String | Item::Quoted => write!(f, "{}", color::Fg(color::LightGreen)),
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
        (self.item, WS, BRP(&self.p), WS).ss_parse(it, res, cf)
    }
}

fn kw(s: &'static str) -> ItemWrap<PKeyWord> {
    ItemWrap {
        p: PKeyWord(s),
        item: Item::Keyword,
    }
}

fn sym<P: SSParser<PConfig>>(p: P) -> ItemWrap<KeyWord<P>> {
    ItemWrap {
        p: KeyWord(p),
        item: Item::Symbol,
    }
}

ss_parser! { WN,
    " \n\t\r".star()
}

ss_parser! {End,
    pl!(WS,ss_or!("\n;".one(),EOI))
}

ss_parser! {(Empties,PConfig),
    PStar(ss_or!((" \n\t\r;").plus(),
            ("#",not("\n;").plus()),
        ))
}

ss_parser! {(ExChannel,PConfig),
    sym(ss_or!( "^^", "^", ""))
}

ss_parser! {(Lines,PConfig),
    (PStar((Empties, FullStatement)),EOI),
}

ss_parser! {(FullStatement,PConfig),
    (Statement,End,Empties)
}

ss_parser! {(Id,PConfig),
    (WS,Item::Ident,common::Ident,WS)
}

ss_parser! {(Idents,PConfig),
    (Item::Ident, PPlus(WS__(common::Ident))),
}

ss_parser! { (Statement,PConfig),
    ss_or!(
        pl!(kw("let"), Idents,WS,sym("="),ArgsS),
        pl!(kw("export"), Idents,WS,sym("="),ArgsS),
        pl!(kw("cd"),WS,ArgsS),
        pl!(kw("for"),PlusUntil(Id,kw("in")),ArgsP,Block),
        pl!(kw("if"),WS,ExprRight,Block,Maybe((WN,kw("else"),Block))),
        pl!(kw("disown"),PExec),
        pl!(sym(". "),WS_(Path)),
        pl!(FailOn(KeyWord(ss_or!("for","export","cd","let","if","else","disown"))),
        ExprRight)
    )
}

ss_parser! {(Block,PConfig),
    pl!(WN,Item::Symbol, "{" ,PStarUntil(pl!(WN,FullStatement),pl!(WN,Item::Symbol,"}")))
}

ss_parser! {(ExprLeft ,PConfig),
    pl!(PExec,Maybe((sym((">",Maybe(">"))),WS,ArgP)))
    //p_list!((Item::Expr) PExec,ws_(pMaybe(p_list!((Item::Command) ExChannel,sym(">"),Maybe(sym(">"),Item::Symbol),ws_(ArgP)),Item::Command)))
}

ss_parser! {(ExprRight,PConfig),
    pl!(ExprLeft,Maybe((WS_(sym(ss_or!("&&","||"))),(WN,ExprRight))))
}

ss_parser! {(ExTarget,PConfig),
    (sym("|"),WS_(PExec))
     //p_list!((Item::Exec) sym("|"),ws_(PExec))
}

ss_parser! {(PConnection,PConfig),
    (ExChannel,ExTarget)
}

ss_parser! {Path,
    pl!(Maybe("~"),PPlus(ss_or!("\\ ",("/._-",Alpha,NumDigit).plus())))
}

ss_parser! {(PExec,PConfig),
    pl!( Item::Command, Path, ArgsS,Maybe(WS_(PConnection)))
}

ss_parser! {(ArgsS ,PConfig),
    PStar((WS,ArgP))
}
ss_parser! {(ArgsP ,PConfig),
    PPlus((WS,Item::Arg,ArgP))
}

ss_parser! { QuotedLitString,
    PPlus(ss_or!(
            not("${}()[]\\\"").plus(),
            ("\\",ss_or!(Any.one(),EOI)),
    ))
}

ss_parser! { (LitString,PConfig),
   PPlus(ss_or!(
            not("#&$|^{}()[]\\\" \n\t<>;").plus(),
            ("\\",ss_or!(ss(Any.one()),EOI)),
    ))
}

ss_parser! {(StringPart,PConfig),
    ss_or!(
        pl!(Item::Symbol, "$[",WS,PExec,WS,Item::Symbol,"]"),
        pl!(Item::Symbol, "$(",WS,PExec,WS,Item::Symbol,")"),
        pl!(Item::Var, "${",WS,(Alpha,NumDigit,"_").plus(),WS,"}"),
        pl!(Item::Var, "$",(Alpha,NumDigit,"_").plus()),
        (Item::String,LitString),
    )
}

ss_parser! {(QuotedStringPart,PConfig),
    ss_or!(
        pl!(Item::Symbol ,sym("$["),WS,PExec,WS,Item::Symbol,"]"),
        pl!(Item::Symbol, sym("$("),WS,PExec,WS,Item::Symbol,")"),
        pl!(Item::Var, "${",WS__((Alpha,NumDigit,"_").plus()),"}"),
        pl!(Item::Var,"$",(Alpha,NumDigit,"_").plus()),
        (Item::String,QuotedLitString),
    )
}

ss_parser! {(ArgP,PConfig),
    ss_or!(
        PRHash,
        PPlus(StringPart),
        pl!(Put(Item::String),sym("\""),Put(Item::Quoted),PStar(QuotedStringPart),sym("\""))
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
                None => {
                    res.push_str(it.str_to(i2.index()));
                    return Ok((i2, None));
                }
            }
        }
        cf.put_item(Item::Quoted, res);
        let raw_start = i2.clone();
        let mut raw_fin = i2.clone();
        'outer: loop {
            match i2.next() {
                Some('"') => {
                    let mut i3 = i2.clone();
                    for _ in 0..nhashes {
                        match i3.next() {
                            Some('#') => {}
                            _ => {
                                raw_fin = i2.clone();
                                continue 'outer;
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
