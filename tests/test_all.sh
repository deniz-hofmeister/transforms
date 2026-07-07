#!/bin/sh
# The full verification gate for this repository; see AGENTS.md "Definition of done".
set -e

cargo build --no-default-features
cargo build
cargo test --no-default-features
cargo test --features serde
cargo test --no-default-features --features serde
cargo test
cargo clippy --all-targets --no-default-features -- -D warnings
cargo clippy --all-targets -- -D warnings
rustup run nightly cargo fmt --check
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
