use std::cmp::Ordering;
use std::ops::{Bound, RangeBounds};
#[derive(Clone, Debug)]
pub struct Cursor {
    pub s: String,
    index: usize,
    line: usize,
    col: usize,
}

macro_rules! lc {
    ($c:expr,$line:expr,$col:expr) => {
        match $c {
            '\n' => {
                $line += 1;
                $col = 0;
            }
            _ => $col += 1,
        }
    };
}

impl Cursor {
    pub fn at_end(s: String) -> Self {
        let index = s.len();
        let mut res = Cursor {
            s,
            line: 0,
            col: 0,
            index: index,
        };
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

    pub fn to_end(&mut self) {
        for c in self.s.chars() {
            lc!(c, self.line, self.col)
        }
        self.index = self.s.len();
    }

    fn fix_index(&mut self) {
        let mut line = 0;
        let mut col = 0;
        for (i, c) in self.s.char_indices() {
            lc!(c, line, col);
            if line == self.line && col == self.col {
                self.index = i;
                return;
            }
        }
    }

    pub fn left(&mut self) -> bool {
        match self.col {
            0 => {
                if (self.line == 0) || (self.index == 0) {
                    self.line = 0;
                    self.index = 0;
                    return false;
                }
                self.line -= 1;
                self.fix_index();
                true
            }
            _ => {
                self.col -= 1;
                for _ in 0..self.index {
                    self.index -= 1;
                    if self.s.get(self.index..).is_some() {
                        return true;
                    }
                }
                self.index = 0;
                true
            }
        }
    }

    pub fn add_char(&mut self, c: char) {
        let mut sc = String::new();
        lc!(c, self.line, self.col);
        sc.push(c);
        self.s.replace_range(self.index..self.index, &sc);
        self.index += sc.len();
        while self.s.get(self.index..).is_none() && self.index > 0 {
            self.index -= 1;
        }
    }

    pub fn del_char(&mut self) {
        let mut s = String::new();
        match self.s.get(self.index..).and_then(|i| i.chars().next()) {
            Some(c) => s.push(c),
            None => return,
        }
        self.s.replace_range(self.index..self.index + s.len(), "");
        self.utf8_hit(self.index - 1);
    }

    pub fn replace_range<R: RangeBounds<usize>>(&mut self, r: R, s2: &str) {
        let len = self.s.len();

        let fit = rel_bound(self.index, &r);
        self.s.replace_range(r, s2);
        match fit {
            Ordering::Greater => {
                self.utf8_hit(self.index + len - self.s.len());
            }
            Ordering::Equal => {
                self.utf8_hit(self.index);
            }
            Ordering::Less => {}
        }
    }
}

pub fn rel_bound<R: RangeBounds<usize>>(n: usize, r: &R) -> Ordering {
    match r.start_bound() {
        Bound::Included(lb) => match n < *lb {
            true => return Ordering::Less,
            _ => {}
        },
        Bound::Excluded(lb) => match n <= *lb {
            true => return Ordering::Less,
            _ => {}
        },
        _ => {}
    }
    match r.end_bound() {
        Bound::Included(lb) => match n > *lb {
            true => Ordering::Greater,
            _ => Ordering::Equal,
        },
        Bound::Excluded(lb) => match n >= *lb {
            true => return Ordering::Greater,
            _ => Ordering::Equal,
        },
        _ => Ordering::Equal,
    }
}
