//! Layout of a parsed clause.
//!
//! Only whitespace is ever changed: every code token is printed exactly once,
//! in the original order. Comments are placed at goal boundaries; a comment in a
//! position the printer cannot handle makes `print_clause` fail so the caller
//! can fall back to the token-level formatter.

use crate::lexer::{Kind, Tok};
use crate::ops::OpTable;
use crate::out::Out;
use crate::parser::{Node, T};

pub struct Comment {
    pub tok: Tok,
    /// Index of the code token that follows the comment.
    pub gap: usize,
    /// The comment starts on the line where the previous code token ends.
    pub same_line: bool,
}

pub const INDENT: usize = 4;

struct Ctx<'a> {
    code: &'a [Tok],
    ops: &'a OpTable,
}

fn is_alnum_name(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_alphabetic())
}

impl<'a> Ctx<'a> {
    fn text(&self, i: usize) -> &str {
        &self.code[i].text
    }

    fn op_of<'n>(&self, n: &'n Node) -> Option<(&str, &'n Node, &'n Node, usize)> {
        match &n.t {
            T::Infix { op, l, r } => Some((self.text(*op), l, r, *op)),
            _ => None,
        }
    }

    fn is_conj(&self, n: &Node) -> bool {
        matches!(&n.t, T::Infix { op, .. } if self.code[*op].kind == Kind::Comma)
    }

    fn is_disj(&self, n: &Node) -> bool {
        matches!(self.op_of(n), Some((";" | "|", ..)))
    }

    fn is_ite(&self, n: &Node) -> bool {
        matches!(self.op_of(n), Some(("->" | "*->", ..)))
    }

    fn is_control(&self, n: &Node) -> bool {
        self.is_disj(n) || self.is_ite(n)
    }

    /// Does laying out this goal require several lines?
    fn forces_break(&self, n: &Node) -> bool {
        if self.is_control(n) {
            return true;
        }
        match &n.t {
            T::Infix { l, r, .. } if self.is_conj(n) => self.forces_break(l) || self.forces_break(r),
            T::Paren(inner) => self.is_control(inner) || (self.is_conj(inner) && self.forces_break(inner)),
            _ => false,
        }
    }

    fn infix_spaced(&self, op: usize, r: &Node) -> bool {
        let t = &self.code[op];
        if t.kind == Kind::Bar || is_alnum_name(&t.text) {
            return true;
        }
        if matches!(t.text.as_str(), ":-" | "-->" | "=>" | "->" | "*->" | ";" | ":=") {
            return true;
        }
        if self.ops.infix(&t.text).is_some_and(|(p, _)| p >= 700) {
            return true;
        }
        if self.code[r.start].is_punct("(") {
            return true;
        }
        t.ws_before || self.code.get(op + 1).is_some_and(|n| n.ws_before)
    }

    fn prefix_spaced(&self, op: usize, arg: &Node) -> bool {
        let name = self.text(op);
        if is_alnum_name(name) || matches!(name, "\\+" | ":-" | "?-") {
            return true;
        }
        let first = &self.code[arg.start];
        if first.is_punct("(") || first.kind == Kind::Num {
            return true;
        }
        first.ws_before
    }

    fn flat(&self, n: &Node, o: &mut Out) {
        let c = self.code;
        match &n.t {
            T::Leaf(i) => o.tok(&c[*i]),
            T::NegNum(a, b) => {
                o.tok(&c[*a]);
                o.tok(&c[*b]);
            }
            T::Empty(a, b) => {
                o.tok(&c[*a]);
                o.tok(&c[*b]);
            }
            T::Compound { name, args } => {
                o.tok(&c[*name]);
                o.tok_tight(&c[name + 1]);
                for (k, a) in args.iter().enumerate() {
                    if k > 0 {
                        o.tok(&c[args[k - 1].end + 1]);
                        o.sp();
                    }
                    self.flat(a, o);
                }
                o.tok(&c[n.end]);
            }
            T::List { items, tail } => {
                o.tok(&c[n.start]);
                for (k, a) in items.iter().enumerate() {
                    if k > 0 {
                        o.tok(&c[items[k - 1].end + 1]);
                        o.sp();
                    }
                    self.flat(a, o);
                }
                if let Some(t) = tail {
                    o.sp();
                    o.tok(&c[items.last().unwrap().end + 1]);
                    o.sp();
                    self.flat(t, o);
                }
                o.tok(&c[n.end]);
            }
            T::Curly(inner) | T::Paren(inner) => {
                o.tok(&c[n.start]);
                self.flat(inner, o);
                o.tok(&c[n.end]);
            }
            T::Infix { op, l, r } => {
                self.flat(l, o);
                if c[*op].kind == Kind::Comma {
                    o.tok(&c[*op]);
                    o.sp();
                } else if self.infix_spaced(*op, r) {
                    o.sp();
                    o.tok(&c[*op]);
                    o.sp();
                } else {
                    o.tok(&c[*op]);
                }
                self.flat(r, o);
            }
            T::Prefix { op, arg } => {
                o.tok(&c[*op]);
                if self.prefix_spaced(*op, arg) {
                    o.sp();
                }
                self.flat(arg, o);
            }
            T::Postfix { op, arg } => {
                self.flat(arg, o);
                o.tok(&c[*op]);
            }
        }
    }

    fn flat_len(&self, n: &Node) -> usize {
        let mut o = Out::new();
        self.flat(n, &mut o);
        o.s.chars().count()
    }

    fn is_atomic(&self, n: &Node) -> bool {
        matches!(n.t, T::Leaf(_) | T::NegNum(..) | T::Empty(..))
    }

    /// Flatten a right-nested chain of the same kind of operator.
    fn chain<'n>(&self, n: &'n Node, pred: &dyn Fn(&Node) -> bool, out: &mut Vec<(&'n Node, Option<usize>)>) {
        let mut cur = n;
        let mut sep = None;
        loop {
            match &cur.t {
                T::Infix { op, l, r } if pred(cur) => {
                    out.push((l, sep));
                    sep = Some(*op);
                    cur = r;
                }
                _ => {
                    out.push((cur, sep));
                    return;
                }
            }
        }
    }
}

pub struct Printer<'a> {
    ctx: Ctx<'a>,
    comments: &'a [Comment],
    used: Vec<bool>,
    width: usize,
    out: Out,
}

impl<'a> Printer<'a> {
    pub fn new(code: &'a [Tok], comments: &'a [Comment], ops: &'a OpTable, width: usize) -> Self {
        Printer { ctx: Ctx { code, ops }, comments, used: vec![false; comments.len()], width, out: Out::new() }
    }

    /// Comments lying between code token `lo` and code token `hi` (exclusive
    /// of `lo`, inclusive of `hi`), split into those on the line of `lo`
    /// (trailing) and the rest (leading). Marks them as used.
    fn gap(&mut self, lo: usize, hi: usize) -> (Vec<usize>, Vec<usize>) {
        let mut trailing = Vec::new();
        let mut leading = Vec::new();
        for (k, c) in self.comments.iter().enumerate() {
            if !self.used[k] && c.gap > lo && c.gap <= hi {
                self.used[k] = true;
                if c.same_line && leading.is_empty() {
                    trailing.push(k);
                } else {
                    leading.push(k);
                }
            }
        }
        (trailing, leading)
    }

    fn has_comments(&self, lo: usize, hi: usize) -> bool {
        self.comments.iter().any(|c| c.gap > lo && c.gap <= hi)
    }

    fn emit_trailing(&mut self, cs: &[usize]) {
        for &k in cs {
            self.out.sp();
            self.out.comment(&self.comments[k].tok);
        }
    }

    /// Comments on their own lines; each is followed by a newline at `ind`.
    fn emit_leading(&mut self, cs: &[usize], ind: usize) {
        for &k in cs {
            self.out.comment(&self.comments[k].tok);
            self.out.nl(ind);
        }
    }

    fn fits(&self, n: &Node, reserve: usize) -> bool {
        self.out.col + self.ctx.flat_len(n) + reserve <= self.width
    }

    fn flat(&mut self, n: &Node) {
        self.ctx.flat(n, &mut self.out);
    }

    fn tok(&mut self, i: usize) {
        self.out.tok(&self.ctx.code[i]);
    }

    pub fn print_clause(mut self, n: &Node) -> Result<String, ()> {
        let code = self.ctx.code;
        let has_end = code.last().is_some_and(|t| t.kind == Kind::End);
        let multiline = code[n.end].end_line > code[n.start].line;
        let inner_comments = self.has_comments(n.start, n.end);
        match &n.t {
            T::Infix { op, l, r } if matches!(self.ctx.text(*op), ":-" | "-->" | "=>") => {
                let (op, l, r) = (*op, &**l, &**r);
                let force = inner_comments || multiline || self.ctx.forces_break(r);
                if !force && self.fits(n, 1) {
                    self.flat(n);
                } else {
                    self.term(l, 0, 3);
                    self.out.sp();
                    self.tok(op);
                    let (tr, ld) = self.gap(l.end, r.start);
                    self.emit_trailing(&tr);
                    self.out.nl(INDENT);
                    self.emit_leading(&ld, INDENT);
                    self.goal(r, INDENT, 1);
                }
            }
            T::Prefix { op, arg } if matches!(self.ctx.text(*op), ":-" | "?-") => {
                let (op, arg) = (*op, &**arg);
                if !inner_comments && !self.ctx.forces_break(arg) && self.fits(n, 1) {
                    self.flat(n);
                } else {
                    self.tok(op);
                    let (tr, ld) = self.gap(op, arg.start);
                    self.emit_trailing(&tr);
                    self.out.nl(INDENT);
                    self.emit_leading(&ld, INDENT);
                    self.goal(arg, INDENT, 1);
                }
            }
            _ => self.term(n, 0, 1),
        }
        let (tr, ld) = self.gap(n.end, code.len());
        if has_end {
            self.tok(code.len() - 1);
        }
        self.emit_trailing(&tr);
        for k in ld {
            self.out.nl(0);
            self.out.comment(&self.comments[k].tok);
        }
        if self.used.iter().all(|u| *u) { Ok(self.out.finish()) } else { Err(()) }
    }

    /// Lay out a goal; the output is positioned at column `ind`.
    fn goal(&mut self, n: &Node, ind: usize, reserve: usize) {
        if self.ctx.is_conj(n) {
            self.conj(n, ind, reserve);
        } else if self.ctx.is_control(n) {
            self.alternatives(n, ind.saturating_sub(INDENT), ind, None, reserve);
        } else if let T::Paren(inner) = &n.t {
            let block = self.ctx.is_control(inner)
                || (self.ctx.is_conj(inner) && (self.has_comments(n.start, n.end) || !self.fits(n, reserve)));
            if block { self.block(n, reserve) } else { self.term(n, ind, reserve) }
        } else {
            self.term(n, ind, reserve);
        }
    }

    fn conj(&mut self, n: &Node, ind: usize, reserve: usize) {
        let ctx = &self.ctx;
        let mut goals = Vec::new();
        ctx.chain(n, &|m| ctx.is_conj(m), &mut goals);
        let code = self.ctx.code;
        let last = goals.len() - 1;
        for (k, (g, sep)) in goals.iter().enumerate() {
            if let Some(sep) = *sep {
                let prev = goals[k - 1].0;
                self.tok(sep);
                let (tr, ld) = self.gap(prev.end, g.start);
                let next_line = ld.first().map_or(code[g.start].line, |&c| self.comments[c].tok.line);
                let after_sep = tr.last().map_or(code[sep].end_line, |&c| self.comments[c].tok.end_line);
                self.emit_trailing(&tr);
                self.out.nl(ind);
                if next_line > after_sep + 1 {
                    self.out.nl(ind);
                }
                self.emit_leading(&ld, ind);
            }
            self.goal(g, ind, if k == last { reserve } else { 1 });
        }
    }

    /// SWI-style block:
    /// ```text
    /// (   Cond
    /// ->  Then
    /// ;   Else
    /// )
    /// ```
    fn block(&mut self, n: &Node, reserve: usize) {
        let T::Paren(inner) = &n.t else { unreachable!() };
        let base = self.out.col;
        self.tok(n.start);
        self.out.pad_to(base + INDENT);
        self.alternatives(inner, base, base + INDENT, Some(n.start), 0);
        let (tr, ld) = self.gap(inner.end, n.end);
        self.emit_trailing(&tr);
        for k in ld {
            self.out.nl(base + INDENT);
            self.out.comment(&self.comments[k].tok);
        }
        self.out.nl(base);
        self.tok(n.end);
        let _ = reserve;
    }

    /// Disjunction / if-then-else layout with separators at `sep_col` and goals
    /// at `goal_col`. The output is positioned at `goal_col` on entry.
    fn alternatives(&mut self, n: &Node, sep_col: usize, goal_col: usize, open: Option<usize>, reserve: usize) {
        let ctx = &self.ctx;
        let mut alts = Vec::new();
        ctx.chain(n, &|m| ctx.is_disj(m), &mut alts);
        let last = alts.len() - 1;
        for (k, (alt, sep)) in alts.iter().enumerate() {
            let before = match sep {
                Some(sep) => {
                    let prev = alts[k - 1].0;
                    let (tr, ld) = self.gap(prev.end, *sep);
                    self.emit_trailing(&tr);
                    for c in ld {
                        self.out.nl(goal_col);
                        self.out.comment(&self.comments[c].tok);
                    }
                    self.out.nl(sep_col);
                    self.tok(*sep);
                    self.out.pad_to(goal_col);
                    Some(*sep)
                }
                None => open,
            };
            if let Some(b) = before {
                let (tr, ld) = self.gap(b, alt.start);
                let all: Vec<usize> = tr.into_iter().chain(ld).collect();
                self.emit_leading(&all, goal_col);
            }
            let r = if k == last { reserve } else { 0 };
            if self.ctx.is_ite(alt) {
                let (_, c, t, op) = self.ctx.op_of(alt).unwrap();
                self.goal(c, goal_col, 0);
                let (tr, ld) = self.gap(c.end, op);
                self.emit_trailing(&tr);
                for cm in ld {
                    self.out.nl(goal_col);
                    self.out.comment(&self.comments[cm].tok);
                }
                self.out.nl(sep_col);
                self.tok(op);
                self.out.pad_to(goal_col);
                let (tr, ld) = self.gap(op, t.start);
                let all: Vec<usize> = tr.into_iter().chain(ld).collect();
                self.emit_leading(&all, goal_col);
                self.goal(t, goal_col, r);
            } else {
                self.goal(alt, goal_col, r);
            }
        }
    }

    /// Lay out a term, breaking it over several lines if it does not fit.
    /// Continuation lines are indented relative to `ind`.
    fn term(&mut self, n: &Node, ind: usize, reserve: usize) {
        if self.fits(n, reserve) {
            self.flat(n);
            return;
        }
        let code = self.ctx.code;
        let inner_ind = ind + INDENT;
        match &n.t {
            T::Compound { name, args } => {
                self.tok(*name);
                self.out.tok_tight(&code[name + 1]);
                if self.hug_last(args, ind, reserve) {
                    self.tok(n.end);
                    return;
                }
                for (k, a) in args.iter().enumerate() {
                    if k > 0 {
                        self.tok(args[k - 1].end + 1);
                    }
                    self.out.nl(inner_ind);
                    self.arg(a, inner_ind, 1);
                }
                self.out.nl(ind);
                self.tok(n.end);
            }
            T::List { items, tail } => {
                self.tok(n.start);
                self.out.nl(inner_ind);
                let fill = items.iter().all(|i| self.ctx.is_atomic(i));
                for (k, a) in items.iter().enumerate() {
                    if k > 0 {
                        self.tok(items[k - 1].end + 1);
                        if fill && self.fits(a, 2) {
                            self.out.sp();
                        } else {
                            self.out.nl(inner_ind);
                        }
                    }
                    self.arg(a, inner_ind, 1);
                }
                if let Some(t) = tail {
                    self.out.sp();
                    self.tok(items.last().unwrap().end + 1);
                    self.out.sp();
                    self.arg(t, inner_ind, 0);
                }
                self.out.nl(ind);
                self.tok(n.end);
            }
            T::Curly(inner) => {
                self.tok(n.start);
                self.out.nl(inner_ind);
                self.goal(inner, inner_ind, 0);
                self.out.nl(ind);
                self.tok(n.end);
            }
            T::Paren(inner) => {
                if self.ctx.is_conj(inner) || self.ctx.is_control(inner) {
                    self.block(n, reserve);
                } else {
                    self.tok(n.start);
                    self.term(inner, ind, reserve + 1);
                    self.tok(n.end);
                }
            }
            T::Infix { op, l, r } => {
                let op = *op;
                if code[op].kind == Kind::Comma {
                    self.term(l, ind, 1);
                    self.tok(op);
                    self.out.nl(ind);
                    self.term(r, ind, reserve);
                    return;
                }
                let spaced = self.ctx.infix_spaced(op, r);
                self.term(l, ind, code[op].text.chars().count() + 1);
                if spaced {
                    self.out.sp();
                }
                self.tok(op);
                if spaced {
                    self.out.sp();
                }
                if self.fits(r, reserve) {
                    self.flat(r);
                } else {
                    self.out.nl(inner_ind);
                    self.term(r, inner_ind, reserve);
                }
            }
            T::Prefix { op, arg } => {
                self.tok(*op);
                if self.ctx.prefix_spaced(*op, arg) {
                    self.out.sp();
                }
                self.term(arg, ind, reserve);
            }
            T::Postfix { op, arg } => {
                self.term(arg, ind, reserve + 1);
                self.tok(*op);
            }
            _ => self.flat(n),
        }
    }

    /// `foo(A, B, [` ... `])`: keep all but the last argument on the opening
    /// line and let the last one (a list or compound) break.
    fn hug_last(&mut self, args: &[Node], ind: usize, reserve: usize) -> bool {
        let last = args.last().unwrap();
        if !matches!(last.t, T::List { .. } | T::Compound { .. }) {
            return false;
        }
        let head: usize = args[..args.len() - 1].iter().map(|a| self.ctx.flat_len(a) + 2).sum();
        // The opening of the last argument must fit too.
        let open = match &last.t {
            T::Compound { name, .. } => self.ctx.code[*name].text.chars().count() + 1,
            _ => 1,
        };
        if self.out.col + head + open > self.width {
            return false;
        }
        let code = self.ctx.code;
        for (k, a) in args[..args.len() - 1].iter().enumerate() {
            self.flat(a);
            self.out.tok(&code[a.end + 1]);
            self.out.sp();
            let _ = k;
        }
        self.term(last, ind, reserve + 1);
        true
    }

    /// An argument of a compound term or list element: a parenthesised
    /// conjunction (a meta-call goal) gets goal layout.
    fn arg(&mut self, n: &Node, ind: usize, reserve: usize) {
        if let T::Paren(inner) = &n.t {
            if (self.ctx.is_conj(inner) || self.ctx.is_control(inner)) && !self.fits(n, reserve) {
                self.block(n, reserve);
                return;
            }
        }
        self.term(n, ind, reserve);
    }
}
