#!/bin/bash -x

cargo clean
cargo build --release && cp target/release/riff /usr/local/bin
ls -lh /usr/local/bin/riff
