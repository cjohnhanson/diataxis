#!/bin/sh
# Everything that must be true before a crate publishes.
#
# Written after diataxis 0.1.0 published with a lib that did not compile
# under `--no-default-features --features store`, which is how two of its
# five consumers take it. `cargo test --all-features` passed, and that was
# read as proof. A feature a consumer uses and the gate never builds is a
# feature nobody tested.
set -eu
CRATE="${1:?usage: prepublish.sh <crate-dir>}"
cd "$CRATE"
TC="${TOOLCHAIN:-1.98.0}"
fail=0
say() { printf '  %-52s %s\n' "$1" "$2"; }

# Every feature combination a consumer can ask for, not only all of them
# at once. Cargo's own `--each-feature` lives in cargo-hack; this covers
# the sets that exist here without another dependency.
feats=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[a-z_-]+ *=/{print $1}' Cargo.toml)
for f in $feats; do
  if rustup run "$TC" cargo check --no-default-features --features "$f" >/dev/null 2>&1; then
    say "check --features $f" ok
  else
    say "check --features $f" FAILS; fail=1
  fi
done
for args in "--no-default-features" "--all-features" ""; do
  if rustup run "$TC" cargo check $args >/dev/null 2>&1; then
    say "check ${args:-<default>}" ok
  else
    say "check ${args:-<default>}" FAILS; fail=1
  fi
done

rustup run "$TC" cargo test --all-features >/dev/null 2>&1 && say "test --all-features" ok || { say "test --all-features" FAILS; fail=1; }
rustup run "$TC" cargo clippy --all-targets --all-features -- -D warnings >/dev/null 2>&1 && say "clippy" ok || { say "clippy" FAILS; fail=1; }
rustup run "$TC" cargo fmt --check >/dev/null 2>&1 && say "fmt" ok || { say "fmt" FAILS; fail=1; }
cargo audit --deny yanked --deny warnings >/dev/null 2>&1 && say "audit" ok || { say "audit" FAILS; fail=1; }

# A clean checkout, which is what a runner and a consumer both get.
tmp=$(mktemp -d)
git archive HEAD | tar -x -C "$tmp"
( cd "$tmp" && rustup run "$TC" cargo metadata --format-version 1 >/dev/null 2>&1 ) \
  && say "clean checkout resolves" ok || { say "clean checkout resolves" FAILS; fail=1; }
( cd "$tmp" && rustup run "$TC" cargo publish --locked --dry-run --allow-dirty >/dev/null 2>&1 ) \
  && say "publish dry run" ok || { say "publish dry run" FAILS; fail=1; }
rm -rf "$tmp"

[ "$fail" -eq 0 ] && echo "  PRE-PUBLISH: ok" || echo "  PRE-PUBLISH: REFUSED"
exit "$fail"
