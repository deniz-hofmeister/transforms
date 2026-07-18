#!/bin/sh
# The full verification gate for this repository; see AGENTS.md "Definition of done".
#
# The gate runs on nightly: rustfmt.toml uses nightly-only options, and one
# pinned toolchain keeps the gate identical on every machine regardless of
# how Rust was installed (rustup, Nix, distro package). Stable and MSRV
# coverage is CI's job (.github/workflows/tests.yml).
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
cargo clippy --all-targets --no-default-features -- -D warnings
cargo clippy --all-targets -- -D warnings
cargo fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
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

# Printed only if every step above succeeded; a truncated run can never be
# mistaken for a pass even when the exit code is swallowed by a pipeline.
echo "GATE PASSED (all steps completed)"
