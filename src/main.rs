use std::io::{Read, Write};
use std::process::ExitCode;

use prolog_fmt::{Options, Outcome, format};

const USAGE: &str = "\
prolog-fmt - Prolog source formatter

USAGE:
    prolog-fmt [OPTIONS] [FILE...]

With no FILE (or FILE '-') reads standard input and writes standard output.

OPTIONS:
    -w, --write        Rewrite files in place
    -c, --check        Exit with status 1 if any file is not formatted
    --width N          Maximum line width (default 120)
    -v, --verbose      Report clauses that could not be fully formatted (stderr)
    -h, --help         Show this help
";

fn main() -> ExitCode {
    // Panics inside the formatter are caught per clause; keep stderr clean.
    std::panic::set_hook(Box::new(|_| {}));
    let mut opts = Options::default();
    let mut write = false;
    let mut check = false;
    let mut verbose = false;
    let mut files = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-w" | "--write" => write = true,
            "-c" | "--check" => check = true,
            "-v" | "--verbose" => verbose = true,
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--width" => match args.next().and_then(|w| w.parse().ok()) {
                Some(w) => opts.width = w,
                None => {
                    eprintln!("prolog-fmt: --width needs a number");
                    return ExitCode::from(2);
                }
            },
            _ if a.starts_with("--width=") => match a["--width=".len()..].parse() {
                Ok(w) => opts.width = w,
                Err(_) => {
                    eprintln!("prolog-fmt: --width needs a number");
                    return ExitCode::from(2);
                }
            },
            _ if a.starts_with('-') && a != "-" => {
                eprintln!("prolog-fmt: unknown option {a}\n\n{USAGE}");
                return ExitCode::from(2);
            }
            _ => files.push(a),
        }
    }
    if files.is_empty() {
        files.push("-".into());
    }

    let mut status = ExitCode::SUCCESS;
    for f in &files {
        let src = if f == "-" {
            let mut s = String::new();
            if let Err(e) = std::io::stdin().read_to_string(&mut s) {
                eprintln!("prolog-fmt: stdin: {e}");
                return ExitCode::from(2);
            }
            s
        } else {
            match std::fs::read_to_string(f) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("prolog-fmt: {f}: {e}");
                    status = ExitCode::from(2);
                    continue;
                }
            }
        };
        let (formatted, notes) = format(&src, &opts);
        if verbose {
            for n in &notes {
                let what = match n.outcome {
                    Outcome::Unparsed => "syntax error, used token-level formatting",
                    Outcome::TokenLayout => "comment placement, used token-level formatting",
                    Outcome::Verbatim => "left unchanged",
                    Outcome::Formatted => continue,
                };
                eprintln!("{f}:{}: {what}", n.line);
            }
        }
        if check {
            if formatted != src {
                println!("{f}");
                status = ExitCode::from(1);
            }
        } else if write && f != "-" {
            if formatted != src {
                if let Err(e) = std::fs::write(f, &formatted) {
                    eprintln!("prolog-fmt: {f}: {e}");
                    status = ExitCode::from(2);
                }
            }
        } else {
            let _ = std::io::stdout().write_all(formatted.as_bytes());
        }
    }
    status
}
