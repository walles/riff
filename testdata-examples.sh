#!/bin/bash

# This script will figure out all testdata example mismatches.
#
# For each mismatch, it will create a /tmp/before.sh shell script that will show
# the .riff-output flavor of the testdata example in moar.
#
# It will also show the actual output.
#
# The idea is that you should run /tmp/before.sh in another tab, and switch
# between tabs to see the differences.
#
# After showing the current output, it will ask whether or not to update the
# .riff-output file.

set -e -o pipefail

WORKFILE=$(mktemp)

for EXPECTED in testdata/*.riff-output; do
    INPUT="${EXPECTED%.riff-output}.diff"
    if [ ! -f "$INPUT" ]; then
        INPUT="${EXPECTED%.riff-output}"
        if [ ! -f "$INPUT" ]; then
            echo "No input file for $EXPECTED"
            exit 1
        fi
    fi

    echo "$INPUT -> $EXPECTED"

    # Create /tmp/before.sh
    cat <<EOF >/tmp/before.sh
#!/bin/bash -e

moar $EXPECTED
EOF
    chmod +x /tmp/before.sh

    # Capture the actual output
    cargo run -- --color=on <"$INPUT" >"$WORKFILE"

    # Is the output different?
    if diff -u "$EXPECTED" "$WORKFILE" >/dev/null; then
        echo "Already up to date, never mind: $EXPECTED"
        continue
    fi

    echo
    read -r -p "Run /tmp/before.sh in another tab, then press Enter to continue"

    moar "$WORKFILE"

    echo
    echo -n "Update $EXPECTED? [y/N] "
    read -r
    if [ "$REPLY" = "y" ]; then
        cp "$WORKFILE" "$EXPECTED"
    fi
    echo
done
