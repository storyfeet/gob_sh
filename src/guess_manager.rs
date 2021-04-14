#[derive(Clone, Debug)]
pub struct GuessManager {
    recent: Vec<String>,
    guesses: Vec<String>,
    p: Option<usize>,
    max_recent: Option<usize>,
}

impl GuessManager {
    pub fn new(max_recent: Option<usize>) -> Self {
        Self {
            recent: Vec::new(),
            guesses: Vec::new(),
            p: None,
            max_recent,
        }
    }

    pub fn set_guesses(&mut self, v: Vec<String>) {
        self.guesses = v;
    }

    pub fn next(&mut self) -> Option<String> {
        let n = match &mut self.p {
            Some(n) => {
                *n += 1;
                *n
            }
            None => {
                self.p = Some(0);
                0
            }
        };
        self.get(n)
    }

    pub fn prev(&mut self) -> Option<String> {
        let n = match &mut self.p {
            None | Some(0) => {
                self.p = None;
                return None;
            }
            Some(n) => {
                *n -= 1;
                *n
            }
        };
        self.get(n)
    }

    pub fn get(&mut self, mut n: usize) -> Option<String> {
        if n < self.guesses.len() {
            return self.guesses.get(n).map(String::clone);
        }
        n -= self.guesses.len();
        if n < self.recent.len() {
            let pos = self.recent.len() - 1 - n;
            return self.recent.get(pos).map(String::clone);
        }
        None
    }

    pub fn add_recent(&mut self, s: String) {
        let mut found = None;
        for (i, v) in self.recent.iter().enumerate() {
            if s == *v {
                found = Some(i);
                break;
            }
        }
        match found {
            Some(n) => {
                self.recent[n..].rotate_left(1);
            }
            None => {
                self.recent.push(s);
                if (&self.max_recent)
                    .map(|v| self.recent.len() > v)
                    .unwrap_or(false)
                {
                    self.recent.remove(0);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.guesses.clear();
        self.p = None;
    }
}
