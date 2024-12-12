#!/bin/sh
set -e 

cargo build --verbose
cargo test --verbose
cargo test --verbose --features async -- async
cargo run --example sync_minimal
cargo run --example sync_polling
cargo run --example async_await --features async
cargo bench
cargo bench --features async

