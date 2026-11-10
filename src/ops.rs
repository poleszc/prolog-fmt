//! Operator table (SWI-Prolog defaults plus common library operators).

use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Assoc {
    Xfx,
    Xfy,
    Yfx,
    Fy,
    Fx,
    Xf,
    Yf,
}

impl Assoc {
    pub fn parse(s: &str) -> Option<Assoc> {
        Some(match s {
            "xfx" => Assoc::Xfx,
            "xfy" => Assoc::Xfy,
            "yfx" => Assoc::Yfx,
            "fy" => Assoc::Fy,
            "fx" => Assoc::Fx,
            "xf" => Assoc::Xf,
            "yf" => Assoc::Yf,
            _ => return None,
        })
    }
}

#[derive(Clone, Default)]
pub struct OpTable {
    prefix: HashMap<String, (u16, Assoc)>,
    infix: HashMap<String, (u16, Assoc)>,
    postfix: HashMap<String, (u16, Assoc)>,
}

const DEFAULT_OPS: &[(u16, &str, &[&str])] = &[
    (1200, "xfx", &[":-", "-->", "=>"]),
    (1200, "fx", &[":-", "?-"]),
    (1100, "xfy", &[";", "|"]),
    (1105, "xfy", &["|"]),
    (1050, "xfy", &["->", "*->"]),
    (1000, "xfy", &[","]),
    (990, "xfx", &[":="]),
    (900, "fy", &["\\+"]),
    (
        700,
        "xfx",
        &[
            "=", "\\=", "==", "\\==", "@<", "@>", "@=<", "@>=", "=..", "is", "=:=", "=\\=", "<", ">", "=<", ">=",
            ">:<", ":<", "as",
        ],
    ),
    (600, "xfy", &[":"]),
    (500, "yfx", &["+", "-", "/\\", "\\/", "xor"]),
    (500, "fx", &["?"]),
    (400, "yfx", &["*", "/", "//", "rdiv", "<<", ">>", "mod", "rem", "div", "divmod"]),
    (200, "xfx", &["**"]),
    (200, "xfy", &["^"]),
    (200, "fy", &["-", "+", "\\"]),
    (100, "yfx", &["."]),
    (1, "fx", &["$"]),
    (
        1150,
        "fx",
        &[
            "dynamic",
            "discontiguous",
            "initialization",
            "meta_predicate",
            "module_transparent",
            "multifile",
            "public",
            "thread_local",
            "table",
        ],
    ),
    // library(clpfd)
    (760, "yfx", &["#<==>"]),
    (750, "xfy", &["#==>"]),
    (750, "yfx", &["#<=="]),
    (740, "yfx", &["#\\/"]),
    (730, "yfx", &["#\\"]),
    (720, "yfx", &["#/\\"]),
    (710, "fy", &["#\\"]),
    (700, "xfx", &["#>", "#<", "#>=", "#=<", "#=", "#\\=", "in", "ins"]),
    (450, "xfx", &[".."]),
];

impl OpTable {
    pub fn swi() -> OpTable {
        let mut t = OpTable::default();
        for (p, a, names) in DEFAULT_OPS {
            let a = Assoc::parse(a).unwrap();
            for n in *names {
                t.add(*p, a, n);
            }
        }
        t
    }

    /// Add (or with priority 0 remove) an operator, as `op/3` does.
    pub fn add(&mut self, p: u16, a: Assoc, name: &str) {
        let map = match a {
            Assoc::Xfx | Assoc::Xfy | Assoc::Yfx => &mut self.infix,
            Assoc::Fy | Assoc::Fx => &mut self.prefix,
            Assoc::Xf | Assoc::Yf => &mut self.postfix,
        };
        if p == 0 {
            map.remove(name);
        } else {
            map.insert(name.to_string(), (p.min(1200), a));
        }
    }

    pub fn prefix(&self, name: &str) -> Option<(u16, Assoc)> {
        self.prefix.get(name).copied()
    }
    pub fn infix(&self, name: &str) -> Option<(u16, Assoc)> {
        self.infix.get(name).copied()
    }
    pub fn postfix(&self, name: &str) -> Option<(u16, Assoc)> {
        self.postfix.get(name).copied()
    }
}
