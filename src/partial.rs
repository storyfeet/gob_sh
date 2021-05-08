use crate::highlight::*;
use bogobble::partial::*;
use bogobble::*;
use std::fmt::Debug;
use transliterate::bo_part::*;
use transliterate::parser::*;
use transliterate::*;

impl SSParser<Highlight> for Item {
    fn ss_parse<'a>(&self, it: &PIter<'a>, res: &mut String, c: &Highlight) -> SSRes<'a> {
        c.put_item(*self, res);
        Ok((it.clone(), None))
    }
}

pub struct ItemWrap<P: SSParser<Highlight>> {
    p: P,
    item: Item,
}

impl<P: SSParser<Highlight>> SSParser<Highlight> for ItemWrap<P> {
    //TODO allow partials
    fn ss_parse<'a>(&self, it: &PIter<'a>, res: &mut String, cf: &Highlight) -> SSRes<'a> {
        (self.item, WS, BRP(&self.p), WS).ss_parse(it, res, cf)
    }
}

fn kw(s: &'static str) -> ItemWrap<PKeyWord> {
    ItemWrap {
        p: PKeyWord(s),
        item: Item::Keyword,
    }
}

ss_parser! { WN,
    " \n\t\r".star()
}

ss_parser! {End,
    pl!(WS,ss_or!("\n;".one(),EOI))
}

ss_parser! {(Empties,Highlight),
    PStar(ss_or!((" \n\t\r;").plus(),
            ("#",not("\n;").plus()),
        ))
}

ss_parser! {(ExChannel,Highlight),
    (Item::Symbol, ss_or!( "^^", "^", ""))
}

ss_parser! {(Lines,Highlight),
    (PStar((Empties, FullStatement)),EOI),
}

ss_parser! {(FullStatement,Highlight),
    (Statement,End,Empties)
}

ss_parser! {(Id,Highlight),
    (WS,Item::Ident,common::Ident,WS)
}

ss_parser! {(Idents,Highlight),
    (Item::Ident, PPlus(WS__(common::Ident))),
}

ss_parser! { (Statement,Highlight),
    ss_or!(
        pl!(kw("let"), Idents,WS,Item::Symbol,"=",ArgsS),
        pl!(kw("export"), Idents,WS,Item::Symbol,"=",ArgsS),
        pl!(kw("cd"),WS,ArgsS),
        pl!(kw("for"),PlusUntil(Id,kw("in")),ArgsP,Block),
        pl!(kw("if"),WS,ExprRight,Block,Maybe((WN,kw("else"),Block))),
        pl!(kw("disown"),PExec),
        pl!(Item::Symbol,". ",WS,Item::Command, Path),
        pl!(FailOn(KeyWord(ss_or!("for","export","cd","let","if","else","disown"))),
        ExprRight)
    )
}

ss_parser! {(Block,Highlight),
    pl!(WN,Item::Symbol, "{" ,PStarUntil(pl!(WN,FullStatement,WN),(Item::Symbol,"}")))
}

ss_parser! {(ExprLeft ,Highlight),
    pl!(PExec,Maybe((Item::Symbol,">",Maybe(">"),WS,ArgP)))
    //p_list!((Item::Expr) PExec,ws_(pMaybe(p_list!((Item::Command) ExChannel,sym(">"),Maybe(sym(">"),Item::Symbol),ws_(ArgP)),Item::Command)))
}

ss_parser! {(ExprRight,Highlight),
    pl!(ExprLeft,Maybe((WS,Item::Symbol,ss_or!("&&","||"),(WN,ExprRight))))
}

ss_parser! {(ExTarget,Highlight),
    (Item::Symbol,"|",WS,PExec)
}

ss_parser! {(PConnection,Highlight),
    (ExChannel,ExTarget)
}

ss_parser! {Path,
    pl!(Maybe("~"),PPlus(ss_or!("\\ ",("/._-",Alpha,NumDigit).plus())))
}

ss_parser! {(PExec,Highlight),
    pl!( Item::Command, Path, ArgsS,Maybe(WS_(PConnection)))
}

ss_parser! {(ArgsS ,Highlight),
    PStar((WS,ArgP))
}
ss_parser! {(ArgsP ,Highlight),
    PPlus((WS,Item::Arg,ArgP))
}

ss_parser! { QuotedLitString,
    PPlus(ss_or!(
            not("${}()[]\\\"").plus(),
            ("\\",ss_or!(Any.one(),EOI)),
    ))
}

ss_parser! { (LitString,Highlight),
   PPlus(ss_or!(
            not("#&$|^{}()[]\\\" \n\t<>;").plus(),
            ("\\",ss_or!(ss(Any.one()),EOI)),
    ))
}

ss_parser! {(StringPart,Highlight),
    ss_or!(
        pl!(Item::Symbol, "$[",WS,PExec,WS,Item::Symbol,"]"),
        pl!(Item::Symbol, "$(",WS,PExec,WS,Item::Symbol,")"),
        pl!(Item::Var, "${",WS,(Alpha,NumDigit,"_").plus(),WS,"}"),
        pl!(Item::Var, "$",(Alpha,NumDigit,"_").plus()),
        (Item::String,LitString),
    )
}

ss_parser! {(QuotedStringPart,Highlight),
    ss_or!(
        pl!(Item::Symbol,Item::Symbol,"$[",WS,PExec,WS,Item::Symbol,"]"),
        pl!(Item::Symbol, Item::Symbol,"$(",WS,PExec,WS,Item::Symbol,")"),
        pl!(Item::Var, "${",WS__((Alpha,NumDigit,"_").plus()),"}"),
        pl!(Item::Var,"$",(Alpha,NumDigit,"_").plus()),
        (Item::String,QuotedLitString),
    )
}

ss_parser! {(ArgP,Highlight),
    ss_or!(
        PRHash,
        PPlus(StringPart),
        pl!(Put(Item::String),Item::Symbol,"\"",Put(Item::Quoted),PStar(QuotedStringPart),Item::Symbol,"\"")
    )
}

/// partial Raw strings eg: r###" Any \ "##  wierd \ string "###
pub struct PRHash;

impl SSParser<Highlight> for PRHash {
    fn ss_parse<'a>(&self, it: &PIter<'a>, res: &mut String, cf: &Highlight) -> SSRes<'a> {
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
