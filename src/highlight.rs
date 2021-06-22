//use crate::parser::Letter;
use crate::partial::*;
use transliterate::parser::BackTo;
//use bogobble::common::*;
use bogobble::*;
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct Highlight {
    mp: BTreeMap<String, String>,
}
impl Highlight {
    pub fn empty() -> Self {
        Highlight {
            mp: BTreeMap::new(),
        }
    }
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        let l = HList.parse_s(s).map_err(|e| e.strung())?;
        let mp = l.into_iter().map(|(k, v)| (k, v.to_string())).collect();
        Ok(Highlight { mp })
    }

    pub fn highlight<'a>(&self, s: &'a str) -> Result<String, bogobble::PErr<'a>> {
        use transliterate::parser::*;
        Lines.ss_convert(s, self)
    }
}

impl BackTo for Highlight {}

impl ParseMark for Highlight {
    fn mark(&self, i: Item, s: &mut String, _: Option<usize>) {
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
    (common::Ident,":",not(",").star()).map(|(a,_,b)|(a,b))
}
