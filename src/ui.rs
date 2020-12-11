pub fn line_count(s: &str, w: usize) -> usize {
    let mut res = 0;
    let mut cw = 0;
    for c in s.chars() {
        match c {
            '\n' => {
                res += 1;
                cw = 0;
            }
            c => {
                if cw + 3 > w {
                    res += 1;
                    cw = 0;
                } else {
                    cw += 1;
                }
            }
        }
    }
    res
}
