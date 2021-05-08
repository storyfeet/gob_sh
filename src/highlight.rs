use bogobble::common::*;
use bogobble::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Write};
use termion::color;

#[derive(Debug, Clone)]
pub struct Highlight {
    mp: BTreeMap<String, String>,
    mark: RefCell<(Item, Option<usize>)>,
}
impl Highlight {
    pub fn empty() -> Self {
        Highlight {
            mp: BTreeMap::new(),
            mark: RefCell::new((Item::Command, None)),
        }
    }
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        let l = HList.parse_s(s).map_err(|e| e.strung())?;
        let mp = l.into_iter().map(|(k, v)| (k, v.to_string())).collect();
        Ok(Highlight {
            mp,
            mark: RefCell::new((Item::Command, None)),
        })
    }

    pub fn put_item(&self, i: Item, s: &mut String) {
        match self.mp.get(i.name()) {
            Some(r) => write!(s, "{}", r),
            None => write!(s, "{}", i),
        }
        .ok();
    }
}

parser! { (HList->Vec<(String,&'a str)>)
    sep_star(HItem,",")
}
parser! {(HItem->(String,&'a str))
    (Ident,":",not(",").star()).map(|(a,_,b)|(a,b))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
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
        }
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Item::Keyword => write!(f, "{}", color::Fg(color::Yellow)),
            //Item::Statement => write!(f, "{}", color::Fg(color::LightMagenta)),
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
