#!/bin/bash

# This script gets run by Travis, see:
# * .travis.yml
# * <http://travis-ci.com/github/walles/riff>

set -ex

# Make sure we're on latest, mostly for Clippy's sake
rustup update
rustup component add clippy rustfmt

# Settings are at the top of main.rs
cargo clippy

# Copied from here:
# <https://docs.travis-ci.com/user/languages/rust/#default-build-script>
cargo build --workspace
cargo test --workspace

# If you have an editor that formats on save this will never be a problem
cargo fmt -- --check

# Verify production crash reporting
cargo build --release
STDERR=$(mktemp -t riff-panic-test.XXX)

echo
echo Writing test crash report here: "$STDERR"...
# The && exit 1 means: If the panic run passes, fail this test run
cargo run --quiet --release -- --please-panic 2> "$STDERR" && exit 1

# Require name and line number for the crash location
grep -E 'src/main\.rs:[0-9]+' "$STDERR" || ( cat "$STDERR" ; exit 1 )

# Require command line arguments
grep -B2 -E -- '--please-panic' "$STDERR" || ( cat "$STDERR" ; exit 1 )

echo
echo Crash reporting tests passed
rm "$STDERR"

echo
echo "All tests passed!"
