#!/bin/bash

# This script gets run by Travis, see:
# * .travis.yml
# * <http://travis-ci.com/github/walles/riff>

set -euxo pipefail

# Make sure we're on latest, mostly for Clippy's sake. On CI, we don't do this
# because GitHub already put some level of this tooling in place.
#
# $CI check from: https://stackoverflow.com/a/13864829/473672
if [[ -z "${CI+x}" ]]; then
    rustup update
fi
rustup component add clippy rustfmt

# Settings are at the top of main.rs
cargo clippy

# Copied from here:
# <https://docs.travis-ci.com/user/languages/rust/#default-build-script>
cargo build --workspace
cargo test --workspace

if [[ -z "${CI+x}" ]]; then
    # Try a Windows build, cross compiles must work
    #
    # Only locally, on CI this should be covered by windows-ci.yml.
    rustup target add x86_64-pc-windows-gnu
    cargo build --release --target=x86_64-pc-windows-gnu
fi

# If you have an editor that formats on save this will never be a problem
cargo fmt -- --check

# Verify production crash reporting
cargo build --release
STDERR=$(mktemp -t riff-panic-test.XXX)

echo
echo Writing test crash report here: "$STDERR"...
# The && exit 1 means: If the panic run passes, fail this test run
cargo run --quiet --release -- --please-panic 2>"$STDERR" && exit 1

# Require name and line number for the crash location
grep -E 'src/main\.rs:[0-9]+' "$STDERR" || (
    cat "$STDERR"
    exit 1
)

# Require command line arguments
grep -B2 -E -- '--please-panic' "$STDERR" || (
    cat "$STDERR"
    exit 1
)

echo
echo Crash reporting tests passed
rm "$STDERR"

# Test diffing two files (myself vs myself)
cargo run --quiet -- "$0" "$0" | wc -l | xargs echo | grep -E "^0$" >/dev/null

# Test case for https://github.com/walles/riff/issues/29
bash -c 'cargo run -- <(echo hej) <(echo nej)' >/dev/null

echo
echo "All tests passed!"
