#!/bin/bash -x

cargo build --release && cp target/release/riff /usr/local/bin
ls -lh /usr/local/bin/riff
