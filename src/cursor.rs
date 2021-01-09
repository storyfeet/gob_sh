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
fn lc(c: char, line: &mut usize, col: &mut usize) {}

impl Cursor {
    pub fn at_end(s: String) -> Self {
        let mut line = 0;
        let mut col = 0;
        for c in s.chars() {
            lc!(c, line, col)
        }
        Cursor {
            s,
            line,
            col,
            index: s.len(),
        }
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

    pub fn left(&self, s: &str) -> bool {
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
            n => {
                self.col -= 1;
                for i in 0..self.index {
                    self.index -= 1;
                    if s.get(self.index..).is_some() {
                        return true;
                    }
                }
                self.index = 0;
                true
            }
        }
    }
}
