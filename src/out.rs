//! Output buffer that tracks the column and inserts a space between two tokens
//! whenever gluing them together would change how they tokenize.

use crate::lexer::{is_symbol_char, is_word_char, Kind, Tok};

pub struct Out {
    pub s: String,
    pub col: usize,
    last: Option<(Kind, char)>,
    line_start: bool,
    must_break: bool,
    cur_ind: usize,
}

/// Would `prev` immediately followed by `next` tokenize differently?
fn needs_space(prev: Kind, a: char, next: Kind, text: &str, tight: bool) -> bool {
    let Some(b) = text.chars().next() else { return false };
    if next == Kind::Punct && text == "(" && !tight && matches!(prev, Kind::Atom | Kind::QAtom) {
        // `foo (` must not turn into the functional notation `foo(`.
        return true;
    }
    if is_word_char(a) && is_word_char(b) {
        return true;
    }
    if is_symbol_char(a) && is_symbol_char(b) {
        return true;
    }
    if prev == Kind::Num && (b == '\'' || (b == '.' && next != Kind::End)) {
        return true;
    }
    if a == b && matches!(a, '\'' | '"' | '`') {
        return true;
    }
    false
}

impl Out {
    pub fn new() -> Out {
        Out { s: String::new(), col: 0, last: None, line_start: true, must_break: false, cur_ind: 0 }
    }

    fn raw(&mut self, t: &str) {
        self.s.push_str(t);
        match t.rfind('\n') {
            Some(p) => self.col = t[p + 1..].chars().count(),
            None => self.col += t.chars().count(),
        }
    }

    fn put(&mut self, kind: Kind, text: &str, tight: bool) {
        if self.must_break {
            self.nl(self.cur_ind);
        }
        if !self.line_start && !self.s.ends_with(' ') {
            if let Some((lk, lc)) = self.last {
                if needs_space(lk, lc, kind, text, tight) {
                    self.raw(" ");
                }
            }
        }
        self.raw(text);
        self.line_start = false;
        self.last = Some((kind, text.chars().last().unwrap_or(' ')));
    }

    pub fn tok(&mut self, t: &Tok) {
        self.put(t.kind, &t.text, false);
    }

    /// Emit `t` with no space before it even if it is `(` after an atom
    /// (used for the `(` of a compound term).
    pub fn tok_tight(&mut self, t: &Tok) {
        self.put(t.kind, &t.text, true);
    }

    pub fn sp(&mut self) {
        if !self.line_start && !self.must_break && !self.s.ends_with(' ') {
            self.raw(" ");
        }
    }

    pub fn nl(&mut self, ind: usize) {
        let trimmed = self.s.trim_end_matches(' ').len();
        self.s.truncate(trimmed);
        self.s.push('\n');
        self.col = 0;
        for _ in 0..ind {
            self.s.push(' ');
        }
        self.col = ind;
        self.line_start = true;
        self.must_break = false;
        self.cur_ind = ind;
    }

    pub fn pad_to(&mut self, c: usize) {
        while self.col < c {
            self.raw(" ");
        }
    }

    pub fn comment(&mut self, t: &Tok) {
        if self.must_break {
            self.nl(self.cur_ind);
        }
        if t.kind == Kind::LineComment {
            self.raw(t.text.trim_end());
            self.must_break = true;
        } else {
            self.raw(&t.text);
        }
        self.line_start = false;
        self.last = Some((t.kind, ' '));
    }

    pub fn finish(mut self) -> String {
        let trimmed = self.s.trim_end().len();
        self.s.truncate(trimmed);
        self.s
    }
}
