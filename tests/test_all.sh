#!/bin/sh
set -e

cargo build 
cargo test 
cargo run --example minimal
cargo run --example polling
cargo run --example full_example
cargo bench
