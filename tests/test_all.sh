#!/bin/sh
set -e

echo "Building with no-std feature..."
cargo build --no-default-features

echo "Building with std feature..."
cargo build

echo "Running tests with no-std feature..."
cargo test --no-default-features

echo "Running tests with std feature..."
cargo test

echo "Running examples..."
cargo run --example no_std_minimal --no-default-features
cargo run --example no_std_full --no-default-features
cargo run --example std_minimal
cargo run --example std_full

echo "Running benchmarks..."
cargo bench --no-default-features
cargo bench

echo "All tests, examples, and benchmarks completed successfully!"