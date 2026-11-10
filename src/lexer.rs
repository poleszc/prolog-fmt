//! Tolerant Prolog tokenizer. It never fails: anything it cannot make sense of
//! becomes a `Kind::Error` token, which makes the enclosing clause be emitted
//! verbatim.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Unquoted atom: letter-digit word, symbol-char sequence, `!` or `;`.
    Atom,
    /// Quoted atom `'...'`.
    QAtom,
    Var,
    Num,
    /// Double-quoted string.
    Str,
    /// Back-quoted string.
    BackQ,
    /// One of `( ) [ ] { }`.
    Punct,
    Comma,
    Bar,
    /// Clause terminator `.` followed by layout, `%` or end of input.
    End,
    LineComment,
    BlockComment,
    /// Unterminated quoted item or unknown character.
    Error,
}

#[derive(Clone, Debug)]
pub struct Tok {
    pub kind: Kind,
    pub text: String,
    pub start: usize,
    pub end: usize,
    /// 0-based line of the first character.
    pub line: usize,
    /// 0-based line of the last character.
    pub end_line: usize,
    /// Whitespace directly precedes this token.
    pub ws_before: bool,
    /// Number of newlines in the whitespace directly preceding this token.
    pub nl_before: usize,
}

impl Tok {
    pub fn is_comment(&self) -> bool {
        matches!(self.kind, Kind::LineComment | Kind::BlockComment)
    }
    pub fn is(&self, kind: Kind, text: &str) -> bool {
        self.kind == kind && self.text == text
    }
    pub fn is_punct(&self, text: &str) -> bool {
        self.is(Kind::Punct, text)
    }
}

pub fn is_symbol_char(c: char) -> bool {
    "+-*/\\^<>=~:.?@#&$".contains(c)
}

pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    line: usize,
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }
    fn peek_at(&self, n: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(n)
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }
    fn eat_while(&mut self, f: impl Fn(char) -> bool) {
        while let Some(c) = self.peek() {
            if !f(c) {
                break;
            }
            self.bump();
        }
    }
    /// True when the character after the current position ends a clause
    /// (layout, `%` or end of input).
    fn end_follows(&self, offset: usize) -> bool {
        match self.peek_at(offset) {
            None => true,
            Some(c) => c.is_whitespace() || c == '%',
        }
    }

    /// Consume a quoted item starting at the opening quote. Returns false when
    /// the item is unterminated (in that case it stops before the newline).
    fn quoted(&mut self, q: char) -> bool {
        self.bump();
        loop {
            match self.peek() {
                None => return false,
                Some('\n') => return false,
                Some(c) if c == q => {
                    self.bump();
                    if self.peek() == Some(q) {
                        self.bump();
                        continue;
                    }
                    return true;
                }
                Some('\\') => {
                    self.bump();
                    // Escaped character, including an escaped newline (line continuation).
                    if self.peek().is_some() {
                        self.bump();
                    }
                }
                Some(_) => {
                    self.bump();
                }
            }
        }
    }

    fn number(&mut self) {
        let first = self.bump().unwrap();
        if first == '0' {
            match self.peek() {
                Some('\'') => {
                    // Character code literal 0'c.
                    self.bump();
                    match self.peek() {
                        Some('\\') => {
                            self.bump();
                            match self.peek() {
                                Some('x') => {
                                    self.bump();
                                    self.eat_while(|c| c.is_ascii_hexdigit());
                                    if self.peek() == Some('\\') {
                                        self.bump();
                                    }
                                }
                                Some(c) if c.is_digit(8) => {
                                    self.eat_while(|c| c.is_digit(8));
                                    if self.peek() == Some('\\') {
                                        self.bump();
                                    }
                                }
                                Some(_) => {
                                    self.bump();
                                }
                                None => {}
                            }
                        }
                        Some('\'') => {
                            self.bump();
                            if self.peek() == Some('\'') {
                                self.bump();
                            }
                        }
                        Some(_) => {
                            self.bump();
                        }
                        None => {}
                    }
                    return;
                }
                Some('x') if self.peek_at(1).is_some_and(|c| c.is_ascii_hexdigit()) => {
                    self.bump();
                    self.eat_while(|c| c.is_ascii_hexdigit());
                    return;
                }
                Some('o') if self.peek_at(1).is_some_and(|c| c.is_digit(8)) => {
                    self.bump();
                    self.eat_while(|c| c.is_digit(8));
                    return;
                }
                Some('b') if self.peek_at(1).is_some_and(|c| c == '0' || c == '1') => {
                    self.bump();
                    self.eat_while(|c| c == '0' || c == '1');
                    return;
                }
                _ => {}
            }
        }
        self.digits();
        if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
            self.digits();
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            let ok = match self.peek_at(1) {
                Some(c) if c.is_ascii_digit() => true,
                Some('+' | '-') => self.peek_at(2).is_some_and(|c| c.is_ascii_digit()),
                _ => false,
            };
            if ok {
                self.bump();
                if matches!(self.peek(), Some('+' | '-')) {
                    self.bump();
                }
                self.digits();
            }
        }
    }

    /// Digits with SWI-Prolog style `_` digit groups (`1_000_000`).
    fn digits(&mut self) {
        loop {
            self.eat_while(|c| c.is_ascii_digit());
            if self.peek() == Some('_') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
                self.bump();
                continue;
            }
            break;
        }
    }
}

pub fn lex(src: &str) -> Vec<Tok> {
    let mut lx = Lexer { src, pos: 0, line: 0 };
    let mut toks = Vec::new();
    loop {
        let ws_start = lx.pos;
        let mut nl = 0;
        while let Some(c) = lx.peek() {
            if !c.is_whitespace() {
                break;
            }
            if c == '\n' {
                nl += 1;
            }
            lx.bump();
        }
        let ws_before = lx.pos > ws_start || toks.is_empty();
        let Some(c) = lx.peek() else { break };
        let start = lx.pos;
        let line = lx.line;
        let mut kind = match c {
            '%' => {
                lx.eat_while(|c| c != '\n');
                Kind::LineComment
            }
            '/' if lx.peek_at(1) == Some('*') => {
                lx.bump();
                lx.bump();
                let mut closed = false;
                while let Some(c) = lx.bump() {
                    if c == '*' && lx.peek() == Some('/') {
                        lx.bump();
                        closed = true;
                        break;
                    }
                }
                if closed { Kind::BlockComment } else { Kind::Error }
            }
            c if c.is_ascii_digit() => {
                lx.number();
                Kind::Num
            }
            c if c == '_' || c.is_uppercase() => {
                lx.eat_while(is_word_char);
                Kind::Var
            }
            c if c.is_alphabetic() => {
                lx.eat_while(is_word_char);
                Kind::Atom
            }
            '\'' => {
                if lx.quoted('\'') { Kind::QAtom } else { Kind::Error }
            }
            '"' => {
                if lx.quoted('"') { Kind::Str } else { Kind::Error }
            }
            '`' => {
                if lx.quoted('`') { Kind::BackQ } else { Kind::Error }
            }
            '(' | ')' | '[' | ']' | '{' | '}' => {
                lx.bump();
                Kind::Punct
            }
            ',' => {
                lx.bump();
                Kind::Comma
            }
            '|' => {
                lx.bump();
                if lx.peek() == Some('|') {
                    lx.bump();
                    Kind::Atom
                } else {
                    Kind::Bar
                }
            }
            '!' | ';' => {
                lx.bump();
                Kind::Atom
            }
            c if is_symbol_char(c) => {
                if c == '.' && lx.end_follows(1) {
                    lx.bump();
                    Kind::End
                } else {
                    lx.eat_while(is_symbol_char);
                    Kind::Atom
                }
            }
            _ => {
                lx.bump();
                Kind::Error
            }
        };
        let text = src[start..lx.pos].to_string();
        if kind == Kind::Atom && text.is_empty() {
            kind = Kind::Error;
        }
        let end_line = line + text.matches('\n').count();
        toks.push(Tok { kind, text, start, end: lx.pos, line, end_line, ws_before, nl_before: nl });
    }
    toks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<(Kind, String)> {
        lex(src).into_iter().map(|t| (t.kind, t.text)).collect()
    }

    #[test]
    fn char_literals() {
        let k = kinds("nawiasy(0'(, 0')). x(0'''). y(0'\\n, 0' ).");
        assert!(k.contains(&(Kind::Num, "0'(".into())));
        assert!(k.contains(&(Kind::Num, "0')".into())));
        assert!(k.contains(&(Kind::Num, "0'''".into())));
        assert!(k.contains(&(Kind::Num, "0'\\n".into())));
        assert!(k.contains(&(Kind::Num, "0' ".into())));
    }

    #[test]
    fn end_detection() {
        let k = kinds("a :- X = '.'.\nb(X) :- X =.. Y.%c\nc :- X = 1.");
        let ends = k.iter().filter(|(k, _)| *k == Kind::End).count();
        assert_eq!(ends, 3);
        assert!(k.contains(&(Kind::Atom, "=..".into())));
    }

    #[test]
    fn unterminated_quote_stops_at_newline() {
        let k = kinds("a('abc).\nb.");
        assert_eq!(k[2].0, Kind::Error);
        assert!(k.contains(&(Kind::Atom, "b".into())));
    }
}
