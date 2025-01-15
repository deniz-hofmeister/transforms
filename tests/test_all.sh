#!/bin/sh
set -e

cargo build 
cargo build --features std
cargo test 
cargo test --features std
cargo run --example no_std_minimal
cargo run --example no_std_full
cargo run --example std_minimal --features std
cargo run --example std_full --features std
cargo bench
cargo bench --features std
