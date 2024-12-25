#!/bin/sh
set -e

cargo build 
cargo test 
cargo test  --features async -- async
cargo run --example sync_minimal
cargo run --example sync_polling
cargo run --example async_await --features async
cargo run --example full_example
cargo bench
cargo bench --features async
