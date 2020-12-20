#!/usr/bin/env python3

import os
import glob
import time
import shutil
import tempfile
import subprocess

BINDIR = os.path.join(
    os.path.dirname(os.path.realpath(__file__)), ".benchmark-binaries"
)
os.makedirs(BINDIR, exist_ok=True)


def gather_binaries():
    """
    Gather binaries to benchmark in BINDIR
    """
    # FIXME: Query git for all Rust versions released and store freshly built
    # binaries for those in BINDIR

    # Build the current version
    subprocess.run(["cargo", "build", "--release"], check=True)
    shutil.copy("target/release/riff", os.path.join(BINDIR, "riff-current"))


def print_timings(binary: str, testdata):
    """
    Run the indicated binary and print timings for it
    """
    t0 = time.time()
    subprocess.check_call(binary, stdin=testdata, stdout=subprocess.DEVNULL)
    t1 = time.time()
    dt_seconds = t1 - t0
    print(f"{dt_seconds * 1000:3.1f}ms: {binary}")


def time_binaries():
    """
    Print timings for all binaries in BINDIR
    """
    print()
    print("=== BENCHMARKING ===")
    with tempfile.NamedTemporaryFile(
        prefix="riff-benchmark", suffix=".gitlog"
    ) as testdata:
        subprocess.check_call(["git", "log", "-p", "master"], stdout=testdata)

        binaries = glob.glob(os.path.join(BINDIR, "*"))
        for binary in binaries:
            print_timings(binary, testdata)
        print_timings("/bin/cat", testdata)


gather_binaries()
time_binaries()
