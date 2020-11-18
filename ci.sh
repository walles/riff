#!/bin/bash

# This script gets run by Travis, see:
# * .travis.yml
# * <http://travis-ci.com/github/walles/riff>

set -ex

# Settings are at the top of main.rs
cargo clippy

# Copied from here:
# <https://docs.travis-ci.com/user/languages/rust/#default-build-script>
cargo build --workspace
cargo test --workspace

# If you have an editor that formats on save this will never be a problem
cargo fmt -- --check

echo
echo "All tests passed!"
