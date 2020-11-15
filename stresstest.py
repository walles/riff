#!/usr/bin/env python3

"""
Stresstest the LCS algorithm.

Result when I tried this on my laptop was that I had to go to 100_000_000 tokens
before getting over 1s of processing time.

So let's assume there are no computational arguments for limiting the input
size.
"""

import sys
import time
import subprocess

token_count = int(sys.argv[1])

removes = b"-" + b"." * token_count
adds = b"+" + b"#" * token_count
t0 = time.time()
print(f"Launching riff with {token_count} tokens mismatching...")
riff = subprocess.Popen(
    ["cargo", "run"], stdin=subprocess.PIPE, stdout=subprocess.DEVNULL
)
assert riff.stdin
riff.stdin.write(removes)
riff.stdin.write(adds)
riff.stdin.close()
riff.wait()
t1 = time.time()
dt_seconds = t1 - t0

print(f"Riff done processing {token_count} differences in {dt_seconds}s")
