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

# Any rustc warning fails the gate (cargo's `build.warnings`). CI's toolchain
# action exports the same in every job that uses it; setting it here keeps
# this script, not the workflow, the place where that strictness is defined.
export CARGO_BUILD_WARNINGS=deny

# The crate has one mode (`no_std` + `alloc`) and one optional feature, so
# every step runs twice: without `serde` and with it. A lint or a failure
# that only exists in the serde path has to stop the gate here.
cargo build
cargo build --features serde
cargo test
cargo test --features serde
# The same two combinations CI's clippy job runs on stable.
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features serde -- -D warnings
cargo fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
# The docs.rs configuration (all features, docsrs cfg) must build too.
RUSTDOCFLAGS="-D warnings --cfg docsrs" cargo doc --no-deps --all-features
# Every example asserts its results and is feature-agnostic: each runs in
# both combinations, and a wrong value fails here.
for example in basic ingest shared_latest cross_time; do
    cargo run --example "$example"
    cargo run --example "$example" --features serde
done
cargo bench -- --test
cargo bench --features serde -- --test
cargo build --target thumbv7em-none-eabihf
cargo build --target thumbv6m-none-eabi
cargo build --target thumbv8m.main-none-eabihf
# serde must stay std-free too: a std leak in the serde path would only
# surface on hosted runners without these.
cargo build --features serde --target thumbv7em-none-eabihf
cargo build --features serde --target thumbv6m-none-eabi
cargo build --features serde --target thumbv8m.main-none-eabihf

# Printed only if every step above succeeded; a truncated run can never be
# mistaken for a pass even when the exit code is swallowed by a pipeline.
echo "GATE PASSED (all steps completed)"
