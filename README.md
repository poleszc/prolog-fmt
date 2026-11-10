# prolog-fmt

A Prolog formatter written in Rust, with a small VSCode extension.

```sh
prolog-fmt file.pl          # print formatted file
prolog-fmt -w *.pl          # rewrite in place
prolog-fmt --check *.pl     # exit 1 if anything would change
prolog-fmt < in.pl > out.pl # stdin -> stdout
prolog-fmt -v file.pl       # report clauses that were not fully formatted
```

Options: `--width N` (default 120).

## Style

- Short clauses stay on one line when they fit: `nat(s(X)) :- nat(X).`
- A clause that was already written over several lines, or that contains
  if-then-else / disjunction or comments, is broken after `:-` with one goal per
  line and 4-space indentation.
- If-then-else and disjunctions use the SWI-Prolog layout:

  ```prolog
  max(X, Y, Z) :-
      (   X >= Y
      ->  Z = X
      ;   Z = Y
      ).
  ```
- `[H | T]`, `f(A, B)`, spaces around operators of priority 700 and above;
  for other symbolic operators (`-`, `/`, `:`, ...) the original spacing is kept.
- Long terms break their arguments one per line; long lists of numbers/atoms
  are filled.
- Runs of blank lines collapse to one. Trailing whitespace is removed.

## Robustness

The formatter only ever changes whitespace. The file is split into clauses at
each terminating `.`, and every clause is handled on its own:

1. parse it and lay it out;
2. if it does not parse (syntax error), re-indent it token by token, keeping its
   line breaks;
3. if that is not possible either (e.g. an unterminated quote), keep it verbatim.

After steps 1 and 2 the output is re-tokenized and compared with the input; any
difference other than whitespace rejects the result. A clause with a missing `.`
is split at the next line that starts in column 0, so the following clauses are
still formatted. `:- op/3` directives are honoured.

## Install

```sh
cargo install --path .
```

VSCode, using the
[Custom Local Formatters](https://marketplace.visualstudio.com/items?itemName=jkillian.custom-local-formatters)
extension, in `settings.json`:

```json
"[prolog]": {
    "editor.defaultFormatter": "jkillian.custom-local-formatters"
},
"customLocalFormatters.formatters": [
    {
        "command": "~/.cargo/bin/prolog-fmt",
        "languages": ["prolog"]
    }
]
```

Format with **Format Document** (Ctrl+Shift+I on Linux). For a different
width use `"command": "~/.cargo/bin/prolog-fmt --width 100"`.

## Tests

```sh
cargo test
```

Tests check idempotency, that only whitespace changes, and that every
truncated prefix of the fixtures (as when formatting mid-edit) is handled.
