#!/bin/sh
# Everything that must be true before this crate publishes.
#
# Written after 0.1.0 published a lib that does not compile under
# `--no-default-features --features store`, which is how two of five
# consumers take it. `cargo test --all-features` passed, and that was
# read as proof. One feature set is not a feature matrix.
#
# The first version of this script parsed feature names out of
# Cargo.toml with awk. A reviewer showed it silently dropped any name
# holding a digit, a capital, quotes or leading whitespace, and never
# tried a combination. A silent skip is the class of failure that
# shipped 0.1.0, so the parsing is cargo-hack's now: it reads features
# from cargo metadata and walks the powerset.
set -eu
TC="${TOOLCHAIN:-1.98.0}"
fail=0
say() { printf '  %-46s %s\n' "$1" "$2"; }
run() {
  label="$1"; shift
  if "$@" >/dev/null 2>&1; then say "$label" ok; else say "$label" FAILS; fail=1; fi
}

if ! cargo hack --version >/dev/null 2>&1; then
  echo "  prepublish: cargo-hack is missing. cargo install cargo-hack" >&2
  exit 1
fi

run "check, every feature alone"  rustup run "$TC" cargo hack check --each-feature --no-dev-deps --all-targets
# The powerset without --all-targets: building every test once per
# combination exhausted memory on a 4-feature crate.
run "check, the feature powerset" rustup run "$TC" cargo hack check --feature-powerset --no-dev-deps
run "test --all-features"         rustup run "$TC" cargo test --all-features
run "clippy"                      rustup run "$TC" cargo clippy --all-targets --all-features -- -D warnings
run "fmt"                         rustup run "$TC" cargo fmt --check
run "audit"                       cargo audit --deny yanked --deny warnings

# A clean checkout is what a runner and a consumer both get. A patch
# file in the working tree hides an unresolvable dependency.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
git archive HEAD | tar -x -C "$tmp"
run "clean checkout resolves" sh -c "cd '$tmp' && rustup run '$TC' cargo metadata --format-version 1"
run "publish dry run"         sh -c "cd '$tmp' && rustup run '$TC' cargo publish --locked --dry-run --allow-dirty"

# A version already on the registry cannot be republished, and finding
# that out during the upload wastes the release.
name=$(rustup run "$TC" cargo metadata --no-deps --format-version 1 | sed -n 's/.*"name":"\([^"]*\)".*/\1/p' | head -1)
ver=$(rustup run "$TC" cargo metadata --no-deps --format-version 1 | sed -n 's/.*"version":"\([^"]*\)".*/\1/p' | head -1)
if curl -sf -H "User-Agent: prepublish" "https://crates.io/api/v1/crates/$name/$ver" >/dev/null 2>&1; then
  say "version $ver is unpublished" FAILS; fail=1
else
  say "version $ver is unpublished" ok
fi

[ "$fail" -eq 0 ] && echo "  PRE-PUBLISH: ok" || echo "  PRE-PUBLISH: REFUSED"
exit "$fail"
