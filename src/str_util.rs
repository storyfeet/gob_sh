use bogobble::traits::CharBool;
use std::slice::SliceIndex;

impl CharStr for str {
    fn get_i(&self, n: usize) -> Option<&str> {
        self.get(n..)
    }
    fn get_r<I: SliceIndex<str>>(&self, i: I) -> Option<&<I as SliceIndex<str>>::Output> {
        self.get(i)
    }
}

pub trait CharStr {
    fn get_i(&self, n: usize) -> Option<&str>;
    fn get_r<I: SliceIndex<str>>(&self, i: I) -> Option<&<I as SliceIndex<str>>::Output>;

    fn char_left(&self, mut n: usize) -> Option<usize> {
        while n > 0 {
            n -= 1;
            if self.get_i(n).is_some() {
                return Some(n);
            }
        }
        None
    }

    fn count_between(&self, a: usize, b: usize) -> Option<usize> {
        Some(self.get_r(a..b)?.chars().fold(0, |a, _| a + 1))
    }

    fn char_right_n_match<C: CharBool>(&self, n: usize, d: usize, pat: C) -> Option<usize> {
        let mut it = self.get_i(n)?.char_indices();
        it.next();
        for _ in 1..d {
            match it.next() {
                Some((i, c)) if pat.char_bool(c) => return Some(i + n),
                _ => {}
            }
        }
        it.next().map(|(i, _)| i + n)
    }

    fn char_right(&self, n: usize) -> Option<usize> {
        self.char_right_n_match(n, 1, "")
    }

    fn char_at(&self, n: usize) -> Option<char> {
        self.get_i(n).and_then(|n| n.chars().next())
    }

    fn prev_match<C: CharBool>(&self, target: C, mut n: usize) -> Option<usize> {
        while n > 0 {
            n -= 1;
            match self.char_at(n) {
                Some(c) if target.char_bool(c) => return Some(n),
                _ => {}
            }
        }
        None
    }

    fn next_match<C: CharBool>(&self, target: C, n: usize) -> Option<usize> {
        for (i, c) in self.get_i(n)?.char_indices() {
            if target.char_bool(c) {
                return Some(n + i);
            }
        }
        None
    }
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn read_length_between_points() {
        assert_eq!("hello world".count_between(2, 4), Some(2));
        assert_eq!("hello 中国and others".count_between(6, 12), Some(2));
    }

    #[test]
    fn step_right_to_line() {
        assert_eq!("hello world".char_right_n_match(2, 2, '\n'), Some(4));
        assert_eq!("hello\nworld".char_right_n_match(2, 10, '\n'), Some(5));
    }

    #[test]
    fn previous_match() {
        assert_eq!("hello world".prev_match(' ', 8), Some(5));
        assert_eq!("he lo world".prev_match(' ', 5), Some(2));
        assert_eq!("he l  world".prev_match(' ', 5), Some(4));
        assert_eq!("hello world".prev_match(' ', 5), None);
    }
}
