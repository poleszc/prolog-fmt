#!/usr/bin/env bash
# Builds both implementations and checks that they produce identical output.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cargo build --release --manifest-path "$root/Cargo.toml" -q
"$root/duckling/build.sh"
rust="$root/target/release/prolog-fmt"
duck="$root/duckling/build/prolog-fmt"
python3 -I "$root/duckling/tests/mutations.py" "$rust" "$duck" "$root"/tests/fixtures/*.pl
for seed in 1 2 3; do
    python3 -I "$root/duckling/tests/generated.py" "$rust" "$duck" 2000 "$seed"
done
