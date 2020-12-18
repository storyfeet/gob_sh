use crate::parser::r_hash;
use bogobble::*;

pub trait Listable: Sized {
    fn p_list<B: OParser<(Pos, bool)>>(self, b: B) -> PosList<Self, B> {
        PosList { a: self, b }
    }
}
impl<A: OParser<Pos>> Listable for Lister<A> {}

impl<A, B> Listable for PosList<A, B> {}

pub trait Endable: Sized {
    fn p_end(self, item: Item) -> ListEnd<Self> {
        ListEnd { p: self, item }
    }
}

impl<'a, P: Parser<'a, Out = (Vec<Pos>, bool)>> Endable for P {}

pub struct ListEnd<P> {
    item: Item,
    p: P,
}

impl<'a, P: Parser<'a, Out = (Vec<Pos>, bool)>> Parser<'a> for ListEnd<P> {
    type Out = Pos;
    fn parse(&self, it: &PIter<'a>) -> ParseRes<'a, Self::Out> {
        self.p.parse(it).map(|(i2, (v, complete), e)| {
            (
                i2,
                Pos {
                    start: it.index(),
                    fin: i2.index(),
                    complete,
                    item: self.item,
                    children: v,
                },
                e,
            )
        })
    }
}

pub struct Lister<P> {
    p: P,
}

impl<'a, P: Parser<'a, Out = Pos>> Parser<'a> for Lister<P> {
    type Out = (Vec<Pos>, bool);
    fn parse(&self, it: &PIter<'a>) -> ParseRes<'a, Self::Out> {
        self.p.parse(it).map_v(|v| (vec![v], v.complete))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Keyword,
    Statement,
    Exec,
    Lit,
    Command,
    Arg,
    Channel,
    Err,
}

pub struct Pos {
    start: Option<usize>,
    fin: Option<usize>,
    complete: bool,
    item: Item,
    children: Vec<Pos>,
}

pub struct PosParse<P> {
    p: P,
    i: Item,
}

fn pos<'a, P: Parser<'a>>(p: P, i: Item) -> PosParse<P> {
    PosParse { p, i }
}

impl<'a, P: Parser<'a, Out = bool>> Parser<'a> for PosParse<P> {
    type Out = Pos;
    fn parse(&self, it: &PIter<'a>) -> ParseRes<'a, Pos> {
        let (i2, complete, e) = self.p.parse(it)?;
        let fin = i2.index();
        let start = it.index();
        Ok((
            i2,
            Pos {
                start,
                fin,
                item: self.i,
                children: Vec::new(),
                complete,
            },
            e,
        ))
    }
}

fn ppos<'a, P: Parser<'a, Out = (Vec<Pos>, bool)>>(p: P, i: Item) -> PosParent<P> {
    PosParent { p, i }
}

pub struct PosParent<P> {
    p: P,
    i: Item,
}

impl<'a, P: Parser<'a, Out = (Vec<Pos>, bool)>> Parser<'a> for PosParent<P> {
    type Out = Pos;
    fn parse(&self, it: &PIter<'a>) -> ParseRes<'a, Pos> {
        let (i2, (children, complete), e) = self.p.parse(it)?;
        let start = it.index();
        let fin = i2.index();
        Ok((
            i2,
            Pos {
                start,
                fin,
                complete,
                item: self.i,
                children,
            },
            e,
        ))
    }
}

pub struct PosList<A, B> {
    a: A,
    b: B,
}

fn plist<'a, A: Parser<'a, Out = (Vec<Pos>, bool)>, B: Parser<'a, Out = (Pos, bool)>>(
    a: A,
    b: B,
) -> PosList<A, B> {
    PosList { a, b }
}

impl<'a, A, B> Parser<'a> for PosList<A, B>
where
    A: Parser<'a, Out = (Vec<Pos>, bool)>,
    B: Parser<'a, Out = (Pos, bool)>,
{
    type Out = (Vec<Pos>, bool);
    fn parse(&self, it: &PIter<'a>) -> ParseRes<'a, Self::Out> {
        let (i2, (mut a, cont), e) = self.a.parse(it)?;
        if !cont {
            return Ok((i2, (a, cont), e));
        }
        let (i3, (b, bcont), e) = self.b.parse(&i2)?;
        a.push(b);
        Ok((i3, (a, bcont), e))
    }
}

macro_rules! p_list (
    (($it:expr)  $s:expr,$($x:expr),* $(,)?) => (Lister{p:$s}$(.p_list($x))*.p_end($it))
);

parser! {(End->())
    ws_(or_ig!("\n;".one(),EOI))
}

parser! {(ExChannel ->Pos)
    pos(or!( "^^", "^", "").asv(true),Item::Channel)
}

parser! {(FullStatement->Pos)
    //TODO
    pos(first(Statement,End).asv(true),Item::Statement)
}

parser! {(Statement->Pos)
    or!(
        p_list!((Item::Statement) pos(keyword("let"),Item::Keyword),plus(ws_(common::Ident)),ws_("="),Args),
        p_list!((Item::Statement) PExec,ws_(maybe((ExChannel,">",exists(">"),ws_(ArgP))))),
    )
}

parser! {(ExTarget->Pos)
     p_list!((Item::Exec) "|",ws_(PExec))
}

parser! {(PConnection->Pos)
    p_list!(ExChannel,ExTarget)
}

parser! {(PExec->Pos)
    p_list!(common::Ident , Args,maybe(ws_(PConnection)))
}

parser! {(Args -> Pos)
    ppos(star(ws_(ArgP)).map(|v|(v,true)),Item::Arg)
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

parser! { (ArgP->Pos)
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
