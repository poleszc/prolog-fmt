#!/usr/bin/env bash
# Builds the Duckling port of prolog-fmt into duckling/build/prolog-fmt.
# DUCKC defaults to the compiler from the copy-duckling checkout.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
DUCKC="${DUCKC:-$HOME/ducktype/copy-duckling/dev/build/bin/duckc}"
"$DUCKC" compile_package "$here/prolog_fmt" -a "$here/build" -n prolog_fmt "$@" | grep -v -E '^\[[0-9]+/[0-9]+\]|^Archiving|^Linking' || true
test -x "$here/build/package_llvm.exe"
cp "$here/build/package_llvm.exe" "$here/build/prolog-fmt"
