#!/bin/bash

# This script gets run by Travis, see:
# * .travis.yml
# * <http://travis-ci.com/github/walles/riff>

set -ex

# Make sure we're on latest
rustup update

# Settings are at the top of main.rs
cargo clippy

# Copied from here:
# <https://docs.travis-ci.com/user/languages/rust/#default-build-script>
cargo build --workspace
cargo test --workspace

# If you have an editor that formats on save this will never be a problem
cargo fmt -- --check

# Verify crash reporting
STDERR=$(mktemp -t riff-panic-test)

echo
echo Writing test crash report here: "$STDERR"...
# The && exit 1 means: If the panic run passes, fail this test run
cargo run --release -- --please-panic 2> "$STDERR" && exit 1

# Require the name of the crashing function
#
# No line numbers on macOS in release builds (even though the ticket is closed)
# on 2020nov27, so we can't check for those:
# <https://github.com/rust-lang/rust/issues/24346>
grep "riff::main" "$STDERR" || ( cat "$STDERR" ; exit 1 )

echo
echo Crash reporting tests passed
rm "$STDERR"

echo
echo "All tests passed!"
