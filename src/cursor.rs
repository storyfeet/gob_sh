use crate::partial::{Item, Lines, ParseMark};
use crate::str_util::CharStr;
use bogobble::partial::ranger::Ranger;
use std::cell::RefCell;
use transliterate::parser::*;

use std::ops::{Bound, RangeBounds};
#[derive(Clone, Debug)]
pub struct Cursor {
    pub s: String,
    index: usize,
}

impl Cursor {
    pub fn at_end(s: String) -> Self {
        let index = s.len();
        let mut res = Cursor { s, index: index };
        res.to_end();
        res
    }

    pub fn utf8_hit(&mut self, mut nv: usize) {
        while self.s.get(nv..).is_some() && nv != 0 {
            nv -= 1;
        }
        self.index = nv;
    }

    pub fn on_s(&self) -> &str {
        &self.s[..self.index]
    }

    pub fn on_to_space(&self) -> &str {
        match self.s.next_match(" \n;", self.index) {
            Some(n) => &self.s[..n],
            None => &self.s,
        }
    }

    pub fn to_line_end(&mut self) {
        self.index = self.s.next_match("\n", self.index).unwrap_or(self.s.len());
    }

    pub fn to_end(&mut self) {
        self.index = self.s.len();
    }

    pub fn up(&mut self) -> bool {
        let this_start = match self.s.prev_match('\n', self.index) {
            Some(n) => n,
            None => return self.s.contains("\n"),
        };
        let prev_start = self.s.prev_match('\n', this_start).unwrap_or(0);
        let df = self.s.count_between(this_start, self.index).unwrap_or(1);
        if let Some(n) = self.s.char_right_n_match(prev_start, df, '\n') {
            self.index = n;
        }
        true
    }

    pub fn down(&mut self) -> bool {
        let next_start = match self.s.next_match('\n', self.index) {
            Some(n) => n,
            None => return self.s.contains("\n"),
        };
        let this_start = self.s.prev_match('\n', self.index).unwrap_or(0);
        let df = self.s.count_between(this_start, self.index).unwrap_or(1);
        match self.s.char_right_n_match(next_start, df, '\n') {
            Some(n) => self.index = n,
            None => self.index = self.s.len(),
        }
        true
    }

    pub fn left(&mut self) -> bool {
        match self.s.char_left(self.index) {
            Some(n) => {
                self.index = n;
                true
            }
            None => false,
        }
    }

    pub fn right(&mut self) -> bool {
        if self.index == self.s.len() {
            return false;
        }
        match self.s.char_right(self.index) {
            Some(n) => self.index = n,
            None => self.index = self.s.len(),
        }
        true
    }

    pub fn add_char(&mut self, c: char) {
        let mut sc = String::new();
        sc.push(c);
        self.s.replace_range(self.index..self.index, &sc);
        self.index += sc.len();
        while self.s.get(self.index..).is_none() && self.index > 0 {
            self.index -= 1;
        }
    }

    pub fn backspace(&mut self) {
        if self.left() {
            self.del_char();
        }
    }

    pub fn del_char(&mut self) {
        let mut s = String::new();
        match self.s.get(self.index..).and_then(|i| i.chars().next()) {
            Some(c) => s.push(c),
            None => return,
        }
        self.s.replace_range(self.index..self.index + s.len(), "");
    }

    pub fn del_line(&mut self) {
        let left = self.s.prev_match('\n', self.index).unwrap_or(0);
        let right = self.s.next_match('\n', self.index).unwrap_or(self.s.len());
        self.s.replace_range(left..right, "");
        self.index = left;
    }

    pub fn replace_range<R: RangeBounds<usize>>(&mut self, r: R, s2: &str) {
        //let len = self.s.len();

        match r.start_bound() {
            Bound::Included(n) | Bound::Excluded(n) => self.index = *n + s2.len(),
            _ => self.index = s2.len(),
        }
        self.s.replace_range(r, s2);
    }
    pub fn is_end(&self) -> bool {
        self.index == self.s.len()
    }

    pub fn get_index(&self) -> usize {
        self.index
    }

    ///@return Result<('commandname', 'item under cursor'),err>
    pub fn item_over(&self) -> anyhow::Result<(CursorItem, CursorItem)> {
        let pf = PosFinder {
            cursor_pos: self.index,
            opens: RefCell::new(Vec::new()),
            res: RefCell::new(CursorItem {
                item: Item::Command,
                start: 0,
                fin: None,
            }),
        };
        match Lines.ss_convert(&self.s, &pf) {
            Ok(_) => Ok((pf.last_command(), pf.res.into_inner())),
            Err(e) => Err(e.strung().into()),
        }
    }
}
#[derive(Clone, Debug)]
pub struct CursorItem {
    pub item: Item,
    pub start: usize,
    pub fin: Option<usize>,
}

impl CursorItem {
    pub fn on_str<'a>(&self, s: &'a str) -> &'a str {
        match &self.fin {
            Some(n) => &s[self.start..*n],
            _ => &s[self.start..],
        }
    }
    pub fn to_ranger(&self) -> Ranger {
        match self.fin {
            Some(n) => Ranger::InEx(self.start, n),
            None => Ranger::InOpen(self.start),
        }
    }
}

pub struct PosFinder {
    cursor_pos: usize,
    opens: RefCell<Vec<CursorItem>>,
    res: RefCell<CursorItem>,
}

impl BackTo for PosFinder {
    fn back(&self, pos: usize) {
        let mut v = self.opens.borrow_mut();
        //println!("opens = {:?}", v);
        if let Some(n) = v.iter().position(|x| x.start > pos) {
            v.truncate(n);
            if n > 0 {
                if let Some(f) = v[n - 1].fin {
                    if f >= pos {
                        v[n - 1].fin = None;
                    }
                }
            }
        } else {
            let l = v.len();
            if l > 0 {
                if let Some(f) = v[l - 1].fin {
                    if f >= pos {
                        v[l - 1].fin = None;
                    }
                }
            }
        }
        //println!("popped = {:?}", v);
    }
}

impl ParseMark for PosFinder {
    fn mark(&self, item: Item, _: &mut String, pos: Option<usize>) {
        match pos {
            Some(n) if n < self.cursor_pos => {
                let mut p = self.res.borrow_mut();
                p.item = item;
                p.start = n;
                match item {
                    Item::Command | Item::Keyword => {
                        let mut v = self.opens.borrow_mut();
                        match v.last_mut() {
                            Some(b) if b.start == n => {
                                b.fin = None;
                            }
                            Some(_) | None => v.push(CursorItem {
                                item,
                                start: n,
                                fin: None,
                            }),
                        }
                    }
                    Item::Close => {
                        self.opens.borrow_mut().pop();
                    }
                    _ => self.end_command(n),
                }
            }
            Some(_) | None => {
                let mut p = self.res.borrow_mut();
                if p.fin == None {
                    p.fin = pos;
                }
            }
        }
    }
}

impl PosFinder {
    fn end_command(&self, pos: usize) {
        let mut v = self.opens.borrow_mut();
        if let Some(i) = v.last_mut() {
            if i.fin == None {
                i.fin = Some(pos)
            }
        }
    }

    fn last_command(&self) -> CursorItem {
        let v = self.opens.borrow();
        let l = v.len();
        if l == 0 {
            return CursorItem {
                item: Item::Command,
                start: 0,
                fin: Some(0),
            };
        }
        v[l - 1].clone()
    }
}
