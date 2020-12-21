#!/usr/bin/env python3

import os
import glob
import time
import shutil
import tempfile
import subprocess

# Number of benchmark iterations to run
ITERATIONS = 60

# Number of throwaway iterations to run before starting the benchmark
WARMUP_RUNS = 10

BINDIR = os.path.join(
    os.path.dirname(os.path.realpath(__file__)), ".benchmark-binaries"
)
os.makedirs(BINDIR, exist_ok=True)


def gather_binaries():
    """
    Gather binaries to benchmark in BINDIR
    """
    # Query git for all Rust versions released and store freshly built binaries
    # for those in BINDIR
    tags = subprocess.check_output(["git", "tag"]).split()
    tags.remove(b"PRERELEASE")

    # Ignore Ruby tags
    rust_tags = filter(lambda tag: not tag.startswith(b"0"), tags)
    rust_tags = filter(lambda tag: not tag.startswith(b"1"), rust_tags)

    # Make sure we binaries for older versions
    with tempfile.TemporaryDirectory(prefix="riff-benchmark") as clonedir:
        subprocess.run(["git", "clone", "-b", "master", ".", clonedir], check=True)
        for tag in rust_tags:
            binary_name = os.path.join(BINDIR, f"riff-{tag.decode()}")
            if os.path.isfile(binary_name):
                continue

            print()
            print(f"Building missing binary: {binary_name}")
            build_binary(clonedir, tag.decode(), binary_name)

    # Build the current version
    print()
    print("Building current sources...")
    subprocess.run(["cargo", "build", "--release"], check=True)
    shutil.copy("target/release/riff", os.path.join(BINDIR, "riff-current"))


def build_binary(clonedir: str, tag: str, binary_name: str):
    """
    In clonedir, check out tag, build a binary and put it in binary_name.
    """
    subprocess.run(["git", "checkout", tag], cwd=clonedir, check=True)
    subprocess.run(["cargo", "build", "--release"], cwd=clonedir, check=True)
    shutil.copy(os.path.join(clonedir, "target", "release", "riff"), binary_name)


def print_timings(binary: str, testdata_filename: str):
    """
    Run the indicated binary and print timings for it
    """

    # Throw away the top and bottom 5%, giving us 90% coverage
    assert ITERATIONS % 20 == 0
    THROW_AWAY_AT_EACH_END = ITERATIONS // 20

    # Do some warmup runs
    for _ in range(WARMUP_RUNS):
        with open(testdata_filename) as testdata:
            subprocess.check_call(binary, stdin=testdata, stdout=subprocess.DEVNULL)

    # Do the actual benchmarking runs
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

        binaries = sorted(glob.glob(os.path.join(BINDIR, "*")))
        for binary in binaries:
            print_timings(binary, testdata.name)
        print_timings("/bin/cat", testdata.name)


gather_binaries()
time_binaries()
