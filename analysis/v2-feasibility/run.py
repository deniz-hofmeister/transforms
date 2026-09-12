#!/usr/bin/env python3
"""Build one standalone client against a selected tree; no added dependencies."""
import argparse
import os
import pathlib
import shutil
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("tree", type=pathlib.Path)
parser.add_argument("output", type=pathlib.Path)
parser.add_argument("--no-default-features", action="store_true")
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
target = args.output.resolve() / "target"
command = ["rustup", "run", "nightly", "cargo", "build", "--release", "--locked",
           "--manifest-path", str(args.tree.resolve() / "Cargo.toml"), "--target-dir", str(target)]
if args.no_default_features:
    command.append("--no-default-features")
compiler = [os.environ.get("RUSTC", shutil.which("rustc"))]
subprocess.run(command, check=True, env={**os.environ, "RUSTC": compiler[0]})
subprocess.run([*compiler, "--edition=2024", "-O",
                str(pathlib.Path(__file__).with_name("probe.rs")), "--extern",
                f"transforms={target / 'release/libtransforms.rlib'}", "-L",
                f"dependency={target / 'release/deps'}", "-o", str(args.output.resolve() / "probe")], check=True)
with (args.output / "trace.txt").open("w") as output:
    subprocess.run([str(args.output.resolve() / "probe"), "trace"], stdout=output, check=True)
