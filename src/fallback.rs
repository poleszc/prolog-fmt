//! Token-level formatter for clauses that do not parse (or whose comments the
//! layout printer cannot place). It keeps the original line breaks, re-indents
//! each line from bracket depth and normalises spacing within lines.

use crate::lexer::{Kind, Tok};
use crate::out::Out;
use crate::printer::INDENT;

fn is_open(t: &Tok) -> bool {
    t.is_punct("(") || t.is_punct("[") || t.is_punct("{")
}
fn is_close(t: &Tok) -> bool {
    t.is_punct(")") || t.is_punct("]") || t.is_punct("}")
}
fn is_lead_op(t: &Tok) -> bool {
    t.kind == Kind::Bar || (t.kind == Kind::Atom && matches!(t.text.as_str(), ";" | "->" | "*->"))
}

pub fn format(toks: &[Tok]) -> String {
    let mut out = Out::new();
    let mut stack: Vec<&str> = Vec::new();
    let mut neck = false;
    // Column of the first token on the current line and whether that token was
    // an opening bracket / leading control operator (SWI-style padding).
    let mut line_ind = 0;
    let mut pad_after_first = false;
    for (k, t) in toks.iter().enumerate() {
        let prev = if k > 0 { Some(&toks[k - 1]) } else { None };
        let line_start = k == 0 || t.nl_before > 0;
        if line_start {
            if k > 0 {
                let mut d = stack.len();
                if is_close(t) || (is_lead_op(t) && stack.last() == Some(&"(")) {
                    d = d.saturating_sub(1);
                }
                line_ind = INDENT * (d + neck as usize);
                if t.nl_before >= 2 {
                    out.nl(0);
                }
                out.nl(line_ind);
            }
            pad_after_first = (is_lead_op(t) && stack.last() == Some(&"(")) || (is_open(t) && toks.get(k + 1).is_some_and(|n| n.ws_before && n.nl_before == 0));
        } else if let Some(p) = prev {
            let first_on_line = k == 1 || p.nl_before > 0;
            if first_on_line && pad_after_first {
                out.pad_to(line_ind + INDENT);
            } else if t.is_comment() {
                out.sp();
            } else if t.kind == Kind::Comma || t.kind == Kind::End || is_close(t) {
                // no space
            } else if p.kind == Kind::Comma || t.kind == Kind::Bar || p.kind == Kind::Bar {
                out.sp();
            } else if is_open(p) {
                // no space
            } else if t.ws_before {
                out.sp();
            }
        }
        if t.is_comment() {
            out.comment(t);
        } else if t.is_punct("(") && !t.ws_before {
            out.tok_tight(t);
        } else {
            out.tok(t);
        }
        if is_open(t) {
            stack.push(&t.text);
        } else if is_close(t) {
            stack.pop();
        }
        if stack.is_empty() && t.kind == Kind::Atom && matches!(t.text.as_str(), ":-" | "-->" | "=>" | "?-") {
            neck = true;
        }
    }
    out.finish()
}
