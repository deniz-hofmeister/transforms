#!/bin/sh
# The full verification gate for this repository; see AGENTS.md "Definition of done".
#
# The gate runs on nightly: rustfmt.toml uses nightly-only options, so this
# script exits rather than run on anything else. No particular nightly is
# pinned — any recent one will do, however Rust was installed (rustup, Nix,
# distro package). Stable and MSRV coverage is CI's job
# (.github/workflows/tests.yml), as is the riscv32imc-unknown-none-elf
# bare-metal build — that target is not built here. CI runs this script
# verbatim in its `gate` job, so this file is the single source of truth
# for what the gate is: extend it here, not in the workflow.
set -e

rustc --version | grep -q nightly || {
    echo "error: the gate requires a nightly toolchain" >&2
    echo "  rustup machines: rustup run nightly tests/test_all.sh" >&2
    exit 1
}

cargo build --no-default-features
cargo build
cargo test --no-default-features
cargo test --features serde
cargo test --no-default-features --features serde
cargo test
# All four feature combinations, the same set CI's clippy job runs: a lint
# that only fires in the serde path has to fail here rather than there, or
# "a green local gate implies green CI clippy" stops being true.
cargo clippy --all-targets --no-default-features -- -D warnings
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --no-default-features --features serde -- -D warnings
cargo clippy --all-targets --features serde -- -D warnings
cargo fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
# The docs.rs configuration (all features, docsrs cfg) must build too.
RUSTDOCFLAGS="-D warnings --cfg docsrs" cargo doc --no-deps --all-features
cargo run --example no_std_minimal --no-default-features
cargo run --example no_std_full --no-default-features
cargo run --example no_std_advanced --no-default-features
cargo run --example std_minimal
cargo run --example std_full
cargo run --example std_advanced
cargo bench --no-default-features -- --test
cargo bench -- --test
cargo build --no-default-features --target thumbv7em-none-eabihf
cargo build --no-default-features --target thumbv6m-none-eabi
cargo build --no-default-features --target thumbv8m.main-none-eabihf
# serde must stay std-free too: a std leak in the serde path would only
# surface on hosted runners without these.
cargo build --no-default-features --features serde --target thumbv7em-none-eabihf
cargo build --no-default-features --features serde --target thumbv6m-none-eabi
cargo build --no-default-features --features serde --target thumbv8m.main-none-eabihf

# Printed only if every step above succeeded; a truncated run can never be
# mistaken for a pass even when the exit code is swallowed by a pipeline.
echo "GATE PASSED (all steps completed)"
