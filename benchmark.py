#!/usr/bin/env python3

import os
import glob
import time
import shutil
import tempfile
import fileinput
import subprocess

# Number of benchmark iterations to run
ITERATIONS = 60

# Confidence interval to present
PRECISION_PERCENT = 50

# Number of throwaway iterations to run before starting the benchmark
WARMUP_RUNS = 10

BINDIR = os.path.join(
    os.path.dirname(os.path.realpath(__file__)), ".benchmark-binaries"
)
os.makedirs(BINDIR, exist_ok=True)


def build_latest_commit(clonedir: str):
    # From: https://stackoverflow.com/a/949391/473672
    latest_sha = (
        subprocess.check_output(["git", "rev-parse", "--verify", "--short", "HEAD"])
        .strip()
        .decode("utf-8")
    )
    latest_commit_bin_name = f"riff-latest-commit-{latest_sha}"

    # Remove any outdated riff-latest-commit built
    for committed in glob.glob(os.path.join(BINDIR, "riff-latest-commit-*")):
        if os.path.basename(committed) != latest_commit_bin_name:
            print(f"Removing build for earlier unreleased commit: {committed}")
            os.remove(committed)

    # Do we already have our build? If so, do nothing.
    full_path = os.path.join(BINDIR, latest_commit_bin_name)
    if os.path.isfile(full_path):
        return

    build_binary(clonedir, latest_sha, full_path)


def replace_line_in_file(filename: str, line_prefix: str, replacement: str) -> None:
    # Inspired by: https://stackoverflow.com/a/290494/473672
    for line in fileinput.input(filename, inplace=True):
        if line.startswith(line_prefix):
            print(replacement)
            continue
        print(line, end="")


def cargo_build(cwd=None):
    main_rs_path = "src/main.rs"
    if cwd:
        main_rs_path = f"{cwd}/{main_rs_path}"

    # Accept any warnings when building. I'd prefer to do this using the cargo
    # build command line, but was unable to figure out how. Better suggestions
    # welcome.
    replace_line_in_file(main_rs_path, "#![deny(warnings)]", "#![allow(warnings)]")

    subprocess.run(["cargo", "build", "--release"], check=True, cwd=cwd)

    # Restore the warnings line
    replace_line_in_file(main_rs_path, "#![allow(warnings)]", "#![deny(warnings)]")


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

        build_latest_commit(clonedir)

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
    cargo_build()
    shutil.copy("target/release/riff", os.path.join(BINDIR, "riff-current"))


def build_binary(clonedir: str, tag: str, binary_name: str):
    """
    In clonedir, check out tag, build a binary and put it in binary_name.
    """
    # Detached HEAD warning disabling:
    # https://stackoverflow.com/a/45652159/473672
    subprocess.run(
        ["git", "-c", "advice.detachedHead=false", "checkout", tag],
        cwd=clonedir,
        check=True,
    )
    cargo_build(cwd=clonedir)
    shutil.copy(os.path.join(clonedir, "target", "release", "riff"), binary_name)


def print_timings(binary: str, testdata_filename: str):
    """
    Run the indicated binary and print timings for it
    """

    # Ensure we throw away an integer number of iterations
    assert ((100 - PRECISION_PERCENT) * ITERATIONS) % 200 == 0
    THROW_AWAY_AT_EACH_END = ((100 - PRECISION_PERCENT) * ITERATIONS) // 200

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
    mid_ms = (from_ms + to_ms) / 2
    spread_ms = to_ms - from_ms
    print(f"{mid_ms:.1f}ms±{spread_ms:.1f}ms: {binary}")


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

        # Do riff-current last: https://stackoverflow.com/a/20320940/473672
        binaries.sort(key=lambda s: s.endswith("riff-current"))

        for binary in binaries:
            print_timings(binary, testdata.name)
        print_timings("/bin/cat", testdata.name)


gather_binaries()
time_binaries()
