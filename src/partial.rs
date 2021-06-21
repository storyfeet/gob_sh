use crate::parser::{Letter, LetterNum};
use bogobble::partial::*;
use bogobble::*;

use std::fmt::{self, Debug, Display};
use termion::*;
use transliterate::bo_part::*;
use transliterate::parser::*;
use transliterate::*;

pub trait ParseMark {
    fn mark(&self, item: Item, s: &mut String, pos: Option<usize>);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    WS,
    Keyword,
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
    String,
    Quoted,
    Expr,
    Close,
    Open,
}

impl Item {
    pub fn name(&self) -> &'static str {
        match self {
            Item::Keyword => "keyword",
            Item::Symbol => "symbol",
            Item::Ident => "ident",
            Item::Path => "path",
            Item::Exec => "exec",
            Item::Esc => "esc",
            Item::Lit => "lit",
            Item::Command => "command",
            Item::Comment => "comment",
            Item::Var => "var",
            Item::Arg => "arg",
            Item::String => "string",
            Item::Quoted => "quoted",
            Item::Expr => "expr",
            Item::WS => "ws",
            Item::Open => "open",
            Item::Close => "close",
        }
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Item::Keyword => write!(f, "{}", color::Fg(color::Yellow)),
            Item::Symbol => write!(f, "{}", color::Fg(color::Blue)),
            Item::Var => write!(f, "{}", color::Fg(color::LightMagenta)),
            Item::Ident | Item::Path => write!(f, "{}", color::Fg(color::Reset)),
            Item::String => write!(f, "{}", color::Fg(color::LightGreen)),
            Item::Lit | Item::Quoted => write!(f, "{}", color::Fg(color::LightYellow)),
            Item::Esc => write!(f, "{}", color::Fg(color::LightBlue)),
            Item::Comment => write!(f, "{}", color::Fg(color::LightBlue)),
            Item::WS | Item::Close => Ok(()),
            _ => write!(f, "{}", color::Fg(color::Reset)),
        }
    }
}
impl<CF: ParseMark> SSParser<CF> for Item {
    fn ss_parse<'a>(&self, it: &PIter<'a>, res: &mut String, c: &CF) -> SSRes<'a> {
        c.mark(*self, res, it.index());
        Ok((it.clone(), None))
    }
}

pub struct ItemWrap<P> {
    p: P,
    item: Item,
}

impl<CF: ParseMark, P: SSParser<CF>> SSParser<CF> for ItemWrap<P> {
    fn ss_parse<'a>(&self, it: &PIter<'a>, res: &mut String, cf: &CF) -> SSRes<'a> {
        (Ws, self.item, BRP(&self.p), Ws).ss_parse(it, res, cf)
    }
}

fn kw(s: &'static str) -> ItemWrap<PKeyWord> {
    ItemWrap {
        p: PKeyWord(s),
        item: Item::Keyword,
    }
}

ss_parser! { Ident:ParseMark,
    ss_or!(
        (Letter.one(),LetterNum.star()),
        pl!('"',Any.except('"').star(),'"'),
    )
}
ss_parser! {Id:ParseMark,
    (Item::Ident,Ident,Ws)
}

ss_parser! {Idents:ParseMark,
    (Ws, PPlus(Id)),
}

ss_parser! { Wn:ParseMark,
    (Item::WS," \n\t\r".star())
}

ss_parser! {Ws:ParseMark,
    (Item::WS," \t".star())
}

ss_parser! {End:ParseMark,
    pl!(Ws,ss_or!("\n;".one(),EOI))
}

ss_parser! {Empties:ParseMark,
    PStar(ss_or!(
            (" \n\t\r;").plus(),
            ("#",not("\n;").plus()),
    ))
}

ss_parser! {ExChannel:ParseMark,
    (Item::Symbol, ss_or!( "^^", "^", ""))
}

ss_parser! {Lines:ParseMark,
    (PStar((Empties, FullStatement)),EOI),
}

ss_parser! {FullStatement:ParseMark,
    (Statement,End,Empties)
}

ss_parser! { Builtin:ParseMark,
    ss_or!(kw("cd"),kw("load"),kw("proglist"),kw("var"),kw("scope_depth"))
}

ss_parser! { Assigner:ParseMark,
    ss_or!(kw("let"),kw("export"),kw("set"),kw("push"))
}

ss_parser! { Statement:ParseMark,
    ss_or!(
        pl!(Assigner,Idents,Item::Symbol,"=", ArgsS),
        pl!(kw("for"),PlusUntil(Id,kw("in")),ArgsP,Block),
        pl!(kw("if"),Ws,ExprRight,Block,Maybe((Wn,kw("else"),Block))),
        pl!(kw("disown"),PExec),
        pl!(Builtin,ArgsS),
        pl!(FailOn(ss_or!(kw("for"),kw("if"),kw("disown"),Builtin,Assigner)),
        ExprRight)
    )
}

ss_parser! {Block:ParseMark,
    pl!(Wn,Item::Symbol, "{" ,PStarUntil(pl!(Wn,FullStatement,Wn),(Item::Symbol,"}")))
}

ss_parser! {ExprLeft:ParseMark,
    pl!(PExec,Maybe((Item::Symbol,">",Maybe(">"),Ws,ArgP)))
    //p_list!((Item::Expr) PExec,ws_(pMaybe(p_list!((Item::Command) ExChannel,sym(">"),Maybe(sym(">"),Item::Symbol),ws_(ArgP)),Item::Command)))
}

ss_parser! {ExprRight:ParseMark,
    pl!(ExprLeft,Maybe((Ws,Item::Symbol,ss_or!("&&","||"),(Wn,ExprRight))))
}

ss_parser! {ExTarget:ParseMark,
    (Item::Symbol,"|",Ws,PExec)
}

ss_parser! {PConnection:ParseMark,
    (ExChannel,ExTarget)
}

ss_parser! {Path:ParseMark,
    pl!(Maybe("~"),PPlus(ss_or!("\\ ",("/",LetterNum).plus())))
}

ss_parser! {PExec:ParseMark,
    pl!( Item::Command, Path, ArgsS,Maybe((Ws,PConnection)))
}

ss_parser! {ArgsS :ParseMark,
    PStar((Ws,ArgP))
}
ss_parser! {ArgsP :ParseMark,
    PPlus((Ws,Item::Arg,ArgP))
}

ss_parser! { QuotedLitString:ParseMark,
    PPlus(ss_or!(
            not("${}()[]\\\"").plus(),
            ("\\",ss_or!(Any.one(),EOI)),
    ))
}

ss_parser! { LitString:ParseMark,
   PPlus(ss_or!(
            not("#&$|^{}()[]\\\" \n\t<>;").plus(),
            ("\\",ss_or!(ss(Any.one()),EOI)),
    ))
}

ss_parser! { Var:ParseMark,
    ss_or!(
        pl!(Item::Var, "${",PSepPlus(pl!(Ws,Id),(Item::Symbol,"|")),Maybe(pl!(Ws,Item::Symbol,",",Ws,ArgP)),Item::Var,"}"),
        pl!(Item::Var, "$",Id),
    )
}

ss_parser! {StringPart:ParseMark,
    ss_or!(
        pl!(Item::Symbol, "$[",Ws,PExec,Ws,Item::Symbol,"]"),
        pl!(Item::Symbol, "$(",Ws,PExec,Ws,Item::Symbol,")"),
        Var,
        (Item::String,LitString),
    )
}

ss_parser! {QuotedStringPart:ParseMark,
    ss_or!(
        pl!(Item::Symbol,Item::Symbol,"$[",Ws,PExec,Ws,Item::Symbol,"]"),
        pl!(Item::Symbol, Item::Symbol,"$(",Ws,PExec,Ws,Item::Symbol,")"),
        Var,
        (Item::Quoted,QuotedLitString),
    )
}

ss_parser! {ArgP:ParseMark,
    ss_or!(
        PRHash,
        pl!(Item::Symbol,"[",ArgsS,Item::Symbol,"]"),
        pl!(Item::Symbol,"{",PSepUntil(pl!(Ws,Id,Item::Symbol,"=",Ws,ArgP),(Item::Symbol,Ws,";"),(Item::Symbol,Ws,"}"))),
        PPlus(StringPart),
        pl!(Put(Item::String),Item::Symbol,"\"",Put(Item::Quoted),PStar(QuotedStringPart),Item::Symbol,"\"")
    )
}

/// partial Raw strings eg: r###" Any \ "##  wierd \ string "###
pub struct PRHash;

impl<CF: ParseMark> SSParser<CF> for PRHash {
    fn ss_parse<'a>(&self, it: &PIter<'a>, res: &mut String, cf: &CF) -> SSRes<'a> {
        let mut i2 = it.clone();
        match i2.next() {
            Some('r') => {}
            None => {
                cf.mark(Item::Symbol, res, it.index());
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
                    cf.mark(Item::Symbol, res, it.index());
                    res.push_str(it.str_to(i2.index()));
                    break;
                }
                Some(_) => return Err(i2.err_s("RawString")),
                None => {
                    match nhashes {
                        0 => cf.mark(Item::String, res, it.index()),
                        _ => cf.mark(Item::Symbol, res, it.index()),
                    }

                    res.push_str(it.str_to(i2.index()));
                    return Ok((i2, None));
                }
            }
        }
        cf.mark(Item::Quoted, res, i2.index());
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
                    cf.mark(Item::Symbol, res, raw_fin.index());
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
