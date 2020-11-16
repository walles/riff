#!/bin/bash

# Run this script to make a new release

set -eu -o pipefail

if uname -a | grep -v Darwin ; then
  >&2 echo "ERROR: This script requires a macOS machine"
  exit 1
fi

# Verify that we're on the right branch
if [ "$(git rev-parse --abbrev-ref HEAD)" != "walles/rust" ] ; then
  echo "ERROR: Releases can be done from the 'walles/rust' branch only"
  exit 1
fi

# Verify there are no outstanding changes
if [ -n "$(git status --porcelain)" ]; then
  echo "ERROR: There are outstanding changes, make sure your working directory is clean before releasing"
  echo
  git status
  exit 1
fi

# Ensure we don't release broken things
./ci.sh

# FIXME: List changes since last release

read -r -p "New version number: " NEW_VERSION_NUMBER

# Validate new version number
if [ -z "$NEW_VERSION_NUMBER" ] ; then
  echo "Empty version number, never mind"
  exit 0
fi

echo Please enter "$NEW_VERSION_NUMBER" again:
read -r -p "  Validate version: " VALIDATE_VERSION_NUMBER

if [ "$NEW_VERSION_NUMBER" != "$VALIDATE_VERSION_NUMBER" ] ; then
  echo "Version numbers mismatch, never mind"
  exit 1
fi

# Get release message from user
cat << EOM

==
You will now get to compose the release description for Github,
write something nice! And remember that the first line is the
subject line for the release.

EOM
read -r -p "Press ENTER when ready: "

git tag --annotate "$NEW_VERSION_NUMBER"

# Build a macOS AMD64 binary
cargo build --release --target=x86_64-apple-darwin
if ! ./target/x86_64-apple-darwin/release/riff --version | grep -E " $NEW_VERSION_NUMBER\$" > /dev/null ; then
    >&2 echo ""
    >&2 echo "ERROR: Version number <$NEW_VERSION_NUMBER> not found in --version output:"
    ./target/x86_64-apple-darwin/release/riff --version
    exit 1
fi
cp "target/x86_64-apple-darwin/release/riff" "riff-$NEW_VERSION_NUMBER-x86_64-apple-darwin"

# FIXME: Build a macOS ARM binary

# Build a Linux-x64 binary on macOS
#
# From: https://timryan.org/2018/07/27/cross-compiling-linux-binaries-from-macos.html
#
# Prerequisites:
# * rustup target add x86_64-unknown-linux-musl
# * brew install FiloSottile/musl-cross/musl-cross
cargo build --release --target=x86_64-unknown-linux-musl
cp "target/x86_64-unknown-linux-musl/release/riff" "riff-$NEW_VERSION_NUMBER-x86_64-unknown-linux-musl"

# Mark new release on Github
git push --tags

cat << EOM

==
Now, go to the Releases page on GitHub:
<https://github.com/walles/riff/releases>

Click your new release.

Click the "Edit tag" button.

Attach your "riff" binaries that was just built to the release:

EOM
ls -lh riff-*

echo
echo 'After uploading that file, press "Publish release".'
echo

read -r -p "Press ENTER when done: "
