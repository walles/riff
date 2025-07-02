#!/bin/bash

# Run this script to make a new release
#
# Run with --dry to test the script without changing anything.

# Prerequisites
# -------------
#
# Cross compile Linux from macOS:
# * brew install FiloSottile/musl-cross/musl-cross
#
# Note config in .cargo/config.toml for this to work.

set -eu -o pipefail

# List available SDKs using "xcodebuild -showsdks"
CROSSBUILD_MACOS_SDK="macosx15.5"

# Git hooks can use this variable to avoid duplicating the CI work we do in here
# anyway.
export RIFF_RELEASING=true

# If this fails, try "xcodebuild -showsdks" to find one that exists
if ! xcrun -sdk $CROSSBUILD_MACOS_SDK --show-sdk-path >/dev/null; then
  echo >&2
  echo >&2 "ERROR: $CROSSBUILD_MACOS_SDK not found, try \"xcodebuild -showsdks\" to find a better one, then update release.sh and try again"
  exit 1
fi

if ! cargo set-version --help >&/dev/null; then
  echo >&2 "ERROR: Must install cargo-edit before releasing: https://github.com/killercup/cargo-edit#installation"
  exit 1
fi

LIVE=true
if [ $# -eq 1 ] && [ "$1" = "--dry" ]; then
  echo "DRY RUN: No changes will be made"
  echo
  LIVE=false
fi

if uname -a | grep -v Darwin; then
  echo >&2 "ERROR: This script must be run on macOS"
  $LIVE && exit 1
fi

# Verify that we're on the right branch
if [ "$(git rev-parse --abbrev-ref HEAD)" != "master" ]; then
  echo "ERROR: Releases can be done from the 'master' branch only"
  $LIVE && exit 1
fi

# Verify there are no outstanding changes
if [ -n "$(git status --porcelain)" ]; then
  echo "ERROR: There are outstanding changes, make sure your working directory is clean before releasing"
  echo
  git status
  $LIVE && exit 1
fi

# Ask user to consider updating the screenshot
cat <<EOM
Please consider updating the screenshot in README.md before releasing.

Scale your window to 92x28, then:
* Get the moar source code: <https://github.com/walles/moar>
* In the moar source code, do: "git show 9c91399309"

Answer yes at this prompt to verify that the Output section is complete.
EOM

read -r -p "Screenshot up to date: " MAYBE_YES
if [ "$MAYBE_YES" != "yes" ]; then
  echo
  echo "Please update the screenshot, then try this script again."
  exit 0
fi

# Ensure we don't release broken things
./ci.sh

# List changes since last release
echo
echo "List of changes since last release:"
git log --color --format="format:%Cgreen%s%Creset (%ad)%n%b" --first-parent "$(git describe --abbrev=0)..HEAD" | cat

echo
echo "=="
echo "Last version number was $(git describe --abbrev=0), enter new version number."
echo "The version number must be on 1.2.3 format."
if ! $LIVE; then
  echo "DRY RUN: Yes, a version number is needed anyway."
fi
read -r -p "New version number: " NEW_VERSION_NUMBER

echo Please enter "$NEW_VERSION_NUMBER" again:
read -r -p "  Validate version: " VALIDATE_VERSION_NUMBER

if [ "$NEW_VERSION_NUMBER" != "$VALIDATE_VERSION_NUMBER" ]; then
  echo "Version numbers mismatch, never mind"
  exit 1
fi

# Validate new version number
cargo set-version --dry-run "$NEW_VERSION_NUMBER"

# Get release message from user
cat <<EOM

==
You will now get to compose the release description for Github,
write something nice! And remember that the first line is the
subject line for the release.

EOM
read -r -p "Press ENTER when ready: "

# Bump version number before we tag the new release
if $LIVE; then
  cargo set-version "$NEW_VERSION_NUMBER"
  cargo check # Required for bumping the Cargo.lock version number

  # This commit will be pushed after the build has been validated, look for
  # "git push" further down in this script.
  git commit -m "Bump version number to $NEW_VERSION_NUMBER" Cargo.*
else
  echo
  echo "DRY RUN: Not bumping Cargo.toml version number."
  echo
fi

if $LIVE; then
  git tag --annotate "$NEW_VERSION_NUMBER"
else
  echo
  echo "DRY RUN: Never mind that message."
  echo
fi

# On $LIVE builds this will be the same as $NEW_VERSION_NUMBER since we just
# made a tag with $NEW_VERSION_NUMBER.
EXPECTED_VERSION_NUMBER=$(git describe --dirty=-modified)

# Build macOS binaries
targets="aarch64-apple-darwin x86_64-apple-darwin"
for target in $targets; do
  rustup target add "$target"

  # From: https://stackoverflow.com/a/66875783/473672
  SDKROOT=$(xcrun -sdk $CROSSBUILD_MACOS_SDK --show-sdk-path) \
  MACOSX_DEPLOYMENT_TARGET=$(xcrun -sdk $CROSSBUILD_MACOS_SDK --show-sdk-platform-version) \
    cargo build --release "--target=$target"
done

# From: https://developer.apple.com/documentation/apple-silicon/building-a-universal-macos-binary#Update-the-Architecture-List-of-Custom-Makefiles
lipo -create \
  -output target/riff-universal-apple-darwin-release \
  target/aarch64-apple-darwin/release/riff \
  target/x86_64-apple-darwin/release/riff

if ! target/riff-universal-apple-darwin-release --version | grep -E " $EXPECTED_VERSION_NUMBER\$" >/dev/null; then
  echo >&2 ""
  echo >&2 "ERROR: Version number <$EXPECTED_VERSION_NUMBER> not found in --version output:"
  target/riff-universal-apple-darwin-release --version
  exit 1
fi
$LIVE && cp "target/riff-universal-apple-darwin-release" "riff-$EXPECTED_VERSION_NUMBER-universal-macos"

# Build a Linux-x64 binary on macOS
#
# From: https://timryan.org/2018/07/27/cross-compiling-linux-binaries-from-macos.html
rustup target add x86_64-unknown-linux-musl
cargo build --release --target=x86_64-unknown-linux-musl
$LIVE && cp "target/x86_64-unknown-linux-musl/release/riff" "riff-$NEW_VERSION_NUMBER-x86_64-linux"

# Build a Linux-ARM64 binary on macOS
#
# From: https://timryan.org/2018/07/27/cross-compiling-linux-binaries-from-macos.html
rustup target add aarch64-unknown-linux-musl
cargo build --release --target=aarch64-unknown-linux-musl
$LIVE && cp "target/aarch64-unknown-linux-musl/release/riff" "riff-$NEW_VERSION_NUMBER-aarch64-linux"

# Build a Windows binary on macOS
#
# From: https://gist.github.com/Mefistophell/9787e1b6d2d9441c16d2ac79d6a505e6
rustup target add x86_64-pc-windows-gnu
cargo build --release --target=x86_64-pc-windows-gnu
$LIVE && cp "target/x86_64-pc-windows-gnu/release/riff.exe" "riff-$NEW_VERSION_NUMBER-x86_64-windows.exe"

# Mark new release on Github. This implicitly triggers Homebrew deployment and
# cargo publishing through deployment.yml.
$LIVE && git push && git push --tags

cat <<EOM

==
EOM

$LIVE && cat <<EOM
Now, create a new release on GitHub:
<https://github.com/walles/riff/releases/new?tag=$NEW_VERSION_NUMBER>

Attach your "riff" binaries that was just built to the release:

EOM
if $LIVE; then
  ls -lh riff-"$NEW_VERSION_NUMBER"-*
else
  ls -lh target/*/release/riff target/*/release/riff.exe
fi

$LIVE && echo
$LIVE && echo 'After uploading that file, press "Publish release".'
$LIVE && echo

$LIVE && read -r -p "Press ENTER when done: "

true # This makes dry runs exit with 0 if they get this far
