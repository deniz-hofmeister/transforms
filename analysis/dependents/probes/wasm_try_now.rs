//! Z10 in roslibrust_transforms.md: `Timestamp::try_now()` on
//! `wasm32-unknown-unknown`. Not a binary of this package. To run it, make a
//! `cdylib` crate with this file as `src/lib.rs` and the same `transforms`
//! path dependency, build it with
//! `cargo build --target wasm32-unknown-unknown`, and call the exports from a
//! wasm host (the 2026-11-06 run used node 24). `try_now_code` traps with
//! `RuntimeError: unreachable`; `from_nanos_ok` returns 1.
use transforms::time::Timestamp;

/// 1 = Ok, 2 = Err (out-of-range clock). A trap means std panicked.
#[unsafe(no_mangle)]
pub extern "C" fn try_now_code() -> u32 {
    match Timestamp::try_now() {
        Ok(_) => 1,
        Err(_) => 2,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn from_nanos_ok() -> u32 {
    u32::from(Timestamp::from_nanos(5).as_nanos() == 5)
}
