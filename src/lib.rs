//! Prolog source formatter.
//!
//! The file is split into clauses at end tokens and each clause is formatted on
//! its own, so a broken clause never affects the rest of the file. For every
//! clause the formatter tries, in order:
//!
//! 1. full layout from a parse tree ([`printer`]),
//! 2. token-level re-indentation ([`fallback`]) if the clause does not parse,
//! 3. the original text, verbatim.
//!
//! The output of steps 1 and 2 is re-tokenized and compared with the input;
//! anything other than a whitespace change is rejected and the next step is
//! used instead.

pub mod fallback;
pub mod lexer;
pub mod ops;
pub mod out;
pub mod parser;
pub mod printer;

use lexer::{Kind, Tok};
use ops::{Assoc, OpTable};
use parser::{Node, Parser, T};
use printer::{Comment, Printer};

pub struct Options {
    pub width: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options { width: 120 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Formatted,
    /// Clause did not parse; token-level formatting was used.
    Unparsed,
    /// Parsed, but comments or layout forced token-level formatting.
    TokenLayout,
    /// Left exactly as written.
    Verbatim,
}

#[derive(Debug, Clone)]
pub struct Note {
    /// 1-based line of the clause start.
    pub line: usize,
    pub outcome: Outcome,
}

enum Item {
    Comment(usize),
    Clause(usize, usize),
}

fn split(toks: &[Tok]) -> Vec<Item> {
    let mut items = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        if toks[i].is_comment() {
            items.push(Item::Comment(i));
            i += 1;
            continue;
        }
        let start = i;
        while i < toks.len() && toks[i].kind != Kind::End {
            i += 1;
        }
        if i < toks.len() {
            i += 1;
            // Comments on the same line as the terminator belong to the clause.
            while i < toks.len() && toks[i].is_comment() && toks[i].nl_before == 0 {
                i += 1;
            }
        }
        items.push(Item::Clause(start, i));
    }
    items
}

/// Code tokens equal and comments equal (ignoring trailing whitespace)?
fn same_tokens(a: &[Tok], b: &[Tok]) -> bool {
    let code = |ts: &[Tok]| -> Vec<(Kind, String)> {
        ts.iter().filter(|t| !t.is_comment()).map(|t| (t.kind, t.text.clone())).collect()
    };
    let comments = |ts: &[Tok]| -> Vec<String> {
        ts.iter().filter(|t| t.is_comment()).map(|t| t.text.trim_end().to_string()).collect()
    };
    code(a) == code(b) && comments(a) == comments(b)
}

fn verify(orig: &[Tok], text: &str) -> bool {
    let new = lexer::lex(text);
    !new.iter().any(|t| t.kind == Kind::Error) && same_tokens(orig, &new)
}

/// Handle `:- op(P, T, Names)` so later clauses parse with the new operators.
fn apply_op_directive(node: &Node, code: &[Tok], ops: &mut OpTable) {
    let T::Prefix { op, arg } = &node.t else { return };
    if code[*op].text != ":-" {
        return;
    }
    let T::Compound { name, args } = &arg.t else { return };
    if code[*name].text != "op" || args.len() != 3 {
        return;
    }
    let (T::Leaf(p), T::Leaf(a)) = (&args[0].t, &args[1].t) else { return };
    let (Ok(p), Some(a)) = (code[*p].text.parse::<u16>(), Assoc::parse(&code[*a].text)) else { return };
    let atom_name = |n: &Node| -> Option<String> {
        match &n.t {
            T::Leaf(i) if code[*i].kind == Kind::Atom => Some(code[*i].text.clone()),
            T::Leaf(i) if code[*i].kind == Kind::QAtom => {
                let t = &code[*i].text;
                Some(t[1..t.len() - 1].replace("''", "'"))
            }
            T::Empty(a, _) if code[*a].text == "[" => Some("[]".into()),
            _ => None,
        }
    };
    match &args[2].t {
        T::List { items, .. } => {
            for it in items {
                if let Some(n) = atom_name(it) {
                    ops.add(p, a, &n);
                }
            }
        }
        _ => {
            if let Some(n) = atom_name(&args[2]) {
                ops.add(p, a, &n);
            }
        }
    }
}

/// Positions where a broken clause probably contains the start of another
/// clause: a line starting in column 0 at bracket depth 0 with an atom (a head)
/// or `:-` (a directive). Typically caused by a missing `.`.
fn recovery_points(src: &str, toks: &[Tok]) -> Vec<usize> {
    let mut depth: i32 = 0;
    let mut points = Vec::new();
    for (k, t) in toks.iter().enumerate() {
        if k > 0
            && depth <= 0
            && t.nl_before > 0
            && matches!(t.kind, Kind::Atom | Kind::QAtom)
            && (t.start == 0 || src.as_bytes()[t.start - 1] == b'\n')
        {
            points.push(k);
        }
        if t.kind == Kind::Error {
            // An unterminated quote may have swallowed closing brackets.
            depth = 0;
        } else if t.kind == Kind::Punct {
            match t.text.as_str() {
                "(" | "[" | "{" => depth += 1,
                _ => depth -= 1,
            }
        }
    }
    points
}

fn format_clause(src: &str, toks: &[Tok], ops: &mut OpTable, opts: &Options) -> (String, Outcome) {
    let (text, outcome) = format_one(src, toks, ops, opts);
    if outcome == Outcome::Formatted {
        return (text, outcome);
    }
    let points = recovery_points(src, toks);
    if points.is_empty() {
        return (text, outcome);
    }
    let mut out = String::new();
    let mut worst = Outcome::Formatted;
    let mut bounds = vec![0];
    bounds.extend(&points);
    bounds.push(toks.len());
    for w in bounds.windows(2) {
        let seg = &toks[w[0]..w[1]];
        if w[0] > 0 {
            out.push_str(if seg[0].nl_before >= 2 { "\n\n" } else { "\n" });
        }
        let (t, o) = format_one(src, seg, ops, opts);
        out.push_str(&t);
        if o != Outcome::Formatted && (worst == Outcome::Formatted || o == Outcome::Verbatim) {
            worst = o;
        }
    }
    // A segment that formatted on its own still means the original clause
    // was broken (it lacked a terminator).
    if worst == Outcome::Formatted {
        worst = Outcome::Unparsed;
    }
    (out, worst)
}

fn format_one(src: &str, toks: &[Tok], ops: &mut OpTable, opts: &Options) -> (String, Outcome) {
    // Last line of defence: a bug in the layout code must never lose the user's
    // text, so a panic leaves the clause as written.
    let mut ops_copy = ops.clone();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let r = format_one_inner(src, toks, &mut ops_copy, opts);
        (r, ops_copy)
    }));
    match r {
        Ok((r, new_ops)) => {
            *ops = new_ops;
            r
        }
        Err(_) => (src[toks[0].start..toks.last().unwrap().end].trim_end().to_string(), Outcome::Verbatim),
    }
}

fn format_one_inner(src: &str, toks: &[Tok], ops: &mut OpTable, opts: &Options) -> (String, Outcome) {
    let verbatim = || src[toks[0].start..toks.last().unwrap().end].trim_end().to_string();
    if toks.iter().any(|t| t.kind == Kind::Error) {
        return (verbatim(), Outcome::Verbatim);
    }
    let mut code = Vec::new();
    let mut comments = Vec::new();
    for t in toks {
        if t.is_comment() {
            let same_line = code.last().is_some_and(|p: &Tok| p.end_line == t.line);
            comments.push(Comment { tok: t.clone(), gap: code.len(), same_line });
        } else {
            code.push(t.clone());
        }
    }
    let mut outcome = Outcome::Unparsed;
    if let Ok(node) = Parser::new(&code, ops).clause() {
        apply_op_directive(&node, &code, ops);
        if let Ok(text) = Printer::new(&code, &comments, ops, opts.width).print_clause(&node) {
            if verify(toks, &text) {
                return (text, Outcome::Formatted);
            }
        }
        outcome = Outcome::TokenLayout;
    }
    let text = fallback::format(toks);
    if verify(toks, &text) { (text, outcome) } else { (verbatim(), Outcome::Verbatim) }
}

pub fn format(src: &str, opts: &Options) -> (String, Vec<Note>) {
    let toks = lexer::lex(src);
    let mut ops = OpTable::swi();
    let mut out = String::new();
    let mut notes = Vec::new();
    for (k, item) in split(&toks).into_iter().enumerate() {
        let first = match item {
            Item::Comment(i) => i,
            Item::Clause(s, _) => s,
        };
        if k > 0 {
            match toks[first].nl_before {
                0 => out.push(' '),
                1 => out.push('\n'),
                _ => out.push_str("\n\n"),
            }
        }
        match item {
            Item::Comment(i) => {
                let t = &toks[i];
                out.push_str(if t.kind == Kind::LineComment { t.text.trim_end() } else { &t.text });
            }
            Item::Clause(s, e) => {
                let (text, outcome) = format_clause(src, &toks[s..e], &mut ops, opts);
                out.push_str(&text);
                if outcome != Outcome::Formatted {
                    notes.push(Note { line: toks[s].line + 1, outcome });
                }
            }
        }
    }
    if !out.is_empty() {
        out.push('\n');
    }
    (out, notes)
}
