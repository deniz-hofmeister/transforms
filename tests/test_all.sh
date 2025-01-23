#!/bin/sh
set -e

cargo build --no-default-features
cargo build
cargo test --no-default-features
cargo test
cargo run --example no_std_minimal --no-default-features
cargo run --example no_std_full --no-default-features
cargo run --example std_minimal
cargo run --example std_full
cargo bench --no-default-features
cargo bench
