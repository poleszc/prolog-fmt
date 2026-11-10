use prolog_fmt::{Options, format, lexer};

fn fmt(src: &str) -> String {
    format(src, &Options::default()).0
}

fn fixtures() -> Vec<(String, String)> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "pl"))
        .map(|p| (p.display().to_string(), std::fs::read_to_string(&p).unwrap()))
        .collect();
    v.sort();
    v
}

/// Code tokens and comment texts, ignoring all whitespace.
fn signature(src: &str) -> Vec<String> {
    lexer::lex(src)
        .into_iter()
        .map(|t| if t.is_comment() || t.kind == lexer::Kind::Error { t.text.trim_end().to_string() } else { t.text })
        .collect()
}

#[test]
fn idempotent() {
    for (name, src) in fixtures() {
        let once = fmt(&src);
        let twice = fmt(&once);
        assert_eq!(once, twice, "not idempotent: {name}");
    }
}

#[test]
fn only_whitespace_changes() {
    for (name, src) in fixtures() {
        assert_eq!(signature(&src), signature(&fmt(&src)), "tokens changed: {name}");
    }
}

#[test]
fn every_prefix_is_safe() {
    // Simulates formatting while typing: any truncation of a valid file must
    // format without panicking and without changing tokens.
    for (name, src) in fixtures() {
        let cuts: Vec<usize> = src.char_indices().map(|(i, _)| i).step_by(7).collect();
        for &cut in &cuts {
            let s = &src[..cut];
            let out = fmt(s);
            assert_eq!(signature(s), signature(&out), "{name} cut at {cut}");
        }
    }
}

#[test]
fn short_clause_stays_on_one_line() {
    assert_eq!(fmt("nat(s(X)):-nat(X)."), "nat(s(X)) :- nat(X).\n");
    assert_eq!(fmt("srodek(E, L) :- scal(P, [E|S], L)."), "srodek(E, L) :- scal(P, [E | S], L).\n");
}

#[test]
fn multiline_clause_stays_broken() {
    let src = "q([L|Ls], S) :-\n  p(Ls, L, X, Y),   \n        q(X, XS).\n";
    assert_eq!(fmt(src), "q([L | Ls], S) :-\n    p(Ls, L, X, Y),\n    q(X, XS).\n");
}

#[test]
fn if_then_else_swi_style() {
    assert_eq!(
        fmt("max(X,Y,Z):-(X>=Y->Z=X;Z=Y)."),
        "max(X, Y, Z) :-\n    (   X >= Y\n    ->  Z = X\n    ;   Z = Y\n    ).\n"
    );
}

#[test]
fn comments_between_goals() {
    let src = "p(X) :- a(X),   % first\n  % b(X),\n c(X).  % last\n";
    assert_eq!(fmt(src), "p(X) :-\n    a(X), % first\n    % b(X),\n    c(X). % last\n");
}

#[test]
fn blank_lines_collapse() {
    assert_eq!(fmt("a.\n\n\n\n\nb.\nc."), "a.\n\nb.\nc.\n");
}

#[test]
fn missing_dot_recovers() {
    let out = fmt("a(X) :- b(X)\nc(Y):-d(Y).\ne:-f.");
    assert_eq!(out, "a(X) :- b(X)\nc(Y) :- d(Y).\ne :- f.\n");
}

#[test]
fn syntax_error_isolated() {
    let out = fmt("a :- b(.\nc:-d.");
    assert!(out.ends_with("c :- d.\n"), "{out}");
}

#[test]
fn char_literals_and_negatives() {
    assert_eq!(fmt("n(0'(,0')).\nx:-X is - 1,Y is -1."), "n(0'(, 0')).\nx :- X is - 1, Y is -1.\n");
}

#[test]
fn op_directive() {
    assert_eq!(fmt(":- op(700, xfx, ===>).\nr(X):-X===>y."), ":- op(700, xfx, ===>).\nr(X) :- X ===> y.\n");
}
