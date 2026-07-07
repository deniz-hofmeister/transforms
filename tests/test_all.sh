#!/bin/sh
# The full verification gate for this repository; see AGENTS.md "Definition of done".
set -e

cargo build --no-default-features
cargo build
cargo test --no-default-features
cargo test
cargo clippy --all-targets --no-default-features
cargo clippy --all-targets
rustup run nightly cargo fmt --check
cargo doc --no-deps
cargo run --example no_std_minimal --no-default-features
cargo run --example no_std_full --no-default-features
cargo run --example no_std_advanced --no-default-features
cargo run --example std_minimal
cargo run --example std_full
cargo run --example std_advanced
cargo bench --no-default-features -- --test
cargo bench -- --test
cargo build --no-default-features --target thumbv7em-none-eabihf
