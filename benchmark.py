#!/usr/bin/env python3

import os
import glob
import time
import shutil
import tempfile
import subprocess

# Number of benchmark iterations to run
ITERATIONS = 60

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


def print_timings(binary: str, testdata_filename: str):
    """
    Run the indicated binary and print timings for it
    """

    # Throw away the top and bottom 5%, giving us 90% coverage
    assert ITERATIONS % 20 == 0
    THROW_AWAY_AT_EACH_END = ITERATIONS // 20

    # FIXME: Do WARMUP_RUNS warmup runs first?

    deltas = []
    for _ in range(ITERATIONS):
        with open(testdata_filename) as testdata:
            t0 = time.time()
            subprocess.check_call(binary, stdin=testdata, stdout=subprocess.DEVNULL)
            t1 = time.time()
            dt_seconds = t1 - t0
            deltas.append(dt_seconds)

    deltas.sort()
    from_ms = deltas[THROW_AWAY_AT_EACH_END] * 1000
    to_ms = deltas[-THROW_AWAY_AT_EACH_END - 1] * 1000
    print(f"{from_ms:.1f}ms-{to_ms:.1f}ms: {binary}")


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
            print_timings(binary, testdata.name)
        print_timings("/bin/cat", testdata.name)


gather_binaries()
time_binaries()
