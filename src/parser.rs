//! Lenient operator-precedence parser. It works on the code tokens of one
//! clause (comments removed) and builds a tree whose nodes refer back to token
//! indices, so the printer can reproduce every token exactly.

use crate::lexer::{Kind, Tok};
use crate::ops::{Assoc, OpTable};

#[derive(Debug, Clone)]
pub enum T {
    /// Atom, variable, number, string.
    Leaf(usize),
    /// `-` immediately followed by a number.
    NegNum(usize, usize),
    /// `[]` or `{}` written as two tokens.
    Empty(usize, usize),
    Compound { name: usize, args: Vec<Node> },
    List { items: Vec<Node>, tail: Option<Box<Node>> },
    Curly(Box<Node>),
    Paren(Box<Node>),
    Infix { op: usize, l: Box<Node>, r: Box<Node> },
    Prefix { op: usize, arg: Box<Node> },
    Postfix { op: usize, arg: Box<Node> },
}

#[derive(Debug, Clone)]
pub struct Node {
    pub t: T,
    /// First and last code token index (inclusive).
    pub start: usize,
    pub end: usize,
}

pub struct Parser<'a> {
    toks: &'a [Tok],
    pos: usize,
    ops: &'a OpTable,
}

type R<T> = Result<T, ()>;

impl<'a> Parser<'a> {
    pub fn new(toks: &'a [Tok], ops: &'a OpTable) -> Self {
        Parser { toks, pos: 0, ops }
    }

    fn peek(&self) -> Option<&'a Tok> {
        self.toks.get(self.pos)
    }
    fn peek2(&self) -> Option<&'a Tok> {
        self.toks.get(self.pos + 1)
    }
    fn expect_punct(&mut self, p: &str) -> R<usize> {
        match self.peek() {
            Some(t) if t.is_punct(p) => {
                self.pos += 1;
                Ok(self.pos - 1)
            }
            _ => Err(()),
        }
    }

    /// Parse a whole clause: a term followed by an end token or end of input.
    pub fn clause(&mut self) -> R<Node> {
        let (n, _) = self.parse(1200, false)?;
        match self.peek() {
            None => Ok(n),
            Some(t) if t.kind == Kind::End && self.pos + 1 == self.toks.len() => Ok(n),
            _ => Err(()),
        }
    }

    /// Does `t` close the current term? (`arg` means we are inside an argument
    /// list or list, where `,` and `|` are separators.)
    fn is_term_end(t: &Tok) -> bool {
        matches!(t.kind, Kind::End | Kind::Comma | Kind::Bar) || t.is_punct(")") || t.is_punct("]") || t.is_punct("}")
    }

    fn infix_of(&self, t: &Tok, arg: bool) -> Option<(u16, Assoc)> {
        match t.kind {
            Kind::Comma if !arg => Some((1000, Assoc::Xfy)),
            Kind::Bar if !arg => Some((1100, Assoc::Xfy)),
            Kind::Atom => self.ops.infix(&t.text),
            _ => None,
        }
    }

    fn parse(&mut self, max: u16, arg: bool) -> R<(Node, u16)> {
        let (mut left, mut lp) = self.primary(max, arg)?;
        loop {
            let Some(t) = self.peek() else { break };
            if let Some((p, a)) = self.infix_of(t, arg) {
                let (lmax, rmax) = match a {
                    Assoc::Xfx => (p - 1, p - 1),
                    Assoc::Xfy => (p - 1, p),
                    _ => (p, p - 1),
                };
                if p <= max && lp <= lmax {
                    let op = self.pos;
                    self.pos += 1;
                    let (r, _) = self.parse(rmax, arg)?;
                    let (start, end) = (left.start, r.end);
                    left = Node { t: T::Infix { op, l: Box::new(left), r: Box::new(r) }, start, end };
                    lp = p;
                    continue;
                }
            }
            if t.kind == Kind::Atom {
                if let Some((p, a)) = self.ops.postfix(&t.text) {
                    let amax = if a == Assoc::Yf { p } else { p - 1 };
                    if p <= max && lp <= amax {
                        let op = self.pos;
                        self.pos += 1;
                        let start = left.start;
                        left = Node { t: T::Postfix { op, arg: Box::new(left) }, start, end: op };
                        lp = p;
                        continue;
                    }
                }
            }
            break;
        }
        Ok((left, lp))
    }

    fn args(&mut self, close: &str) -> R<Vec<Node>> {
        let mut args = Vec::new();
        loop {
            let (a, _) = self.parse(1200, true)?;
            args.push(a);
            match self.peek() {
                Some(t) if t.kind == Kind::Comma => self.pos += 1,
                Some(t) if t.is_punct(close) => {
                    self.pos += 1;
                    return Ok(args);
                }
                _ => return Err(()),
            }
        }
    }

    fn primary(&mut self, max: u16, arg: bool) -> R<(Node, u16)> {
        let i = self.pos;
        let t = self.peek().ok_or(())?;
        let leaf = |p: &mut Self| {
            p.pos += 1;
            Ok((Node { t: T::Leaf(i), start: i, end: i }, 0))
        };
        let functional = self.peek2().is_some_and(|n| n.is_punct("(") && !n.ws_before);
        match t.kind {
            Kind::Var | Kind::Num | Kind::Str | Kind::BackQ => leaf(self),
            Kind::QAtom | Kind::Atom if functional => {
                self.pos += 2;
                let args = self.args(")")?;
                Ok((Node { t: T::Compound { name: i, args }, start: i, end: self.pos - 1 }, 0))
            }
            Kind::QAtom => leaf(self),
            Kind::Atom => {
                let next = self.peek2();
                if t.text == "-" && next.is_some_and(|n| n.kind == Kind::Num && !n.ws_before) {
                    self.pos += 2;
                    return Ok((Node { t: T::NegNum(i, i + 1), start: i, end: i + 1 }, 0));
                }
                if let Some((p, a)) = self.ops.prefix(&t.text) {
                    let as_atom = match next {
                        None => true,
                        Some(n) if Self::is_term_end(n) => true,
                        Some(n) => {
                            n.kind == Kind::Atom
                                && self.infix_of(n, arg).is_some()
                                && self.ops.prefix(&n.text).is_none()
                                && !self.toks.get(i + 2).is_some_and(|m| m.is_punct("(") && !m.ws_before)
                        }
                    };
                    if !as_atom {
                        let p = p.min(max);
                        let amax = if a == Assoc::Fy { p } else { p.saturating_sub(1) };
                        self.pos += 1;
                        let (operand, _) = self.parse(amax, arg)?;
                        let end = operand.end;
                        return Ok((Node { t: T::Prefix { op: i, arg: Box::new(operand) }, start: i, end }, p));
                    }
                }
                leaf(self)
            }
            Kind::Punct => match t.text.as_str() {
                "(" => {
                    self.pos += 1;
                    let (inner, _) = self.parse(1200, false)?;
                    let end = self.expect_punct(")")?;
                    Ok((Node { t: T::Paren(Box::new(inner)), start: i, end }, 0))
                }
                "[" => {
                    self.pos += 1;
                    if let Ok(end) = self.expect_punct("]") {
                        return Ok((Node { t: T::Empty(i, end), start: i, end }, 0));
                    }
                    let mut items = Vec::new();
                    let mut tail = None;
                    loop {
                        let (it, _) = self.parse(1200, true)?;
                        items.push(it);
                        match self.peek() {
                            Some(t) if t.kind == Kind::Comma => self.pos += 1,
                            Some(t) if t.kind == Kind::Bar => {
                                self.pos += 1;
                                let (tl, _) = self.parse(1200, true)?;
                                tail = Some(Box::new(tl));
                                break;
                            }
                            _ => break,
                        }
                    }
                    let end = self.expect_punct("]")?;
                    Ok((Node { t: T::List { items, tail }, start: i, end }, 0))
                }
                "{" => {
                    self.pos += 1;
                    if let Ok(end) = self.expect_punct("}") {
                        return Ok((Node { t: T::Empty(i, end), start: i, end }, 0));
                    }
                    let (inner, _) = self.parse(1200, false)?;
                    let end = self.expect_punct("}")?;
                    Ok((Node { t: T::Curly(Box::new(inner)), start: i, end }, 0))
                }
                _ => Err(()),
            },
            _ => Err(()),
        }
    }
}
