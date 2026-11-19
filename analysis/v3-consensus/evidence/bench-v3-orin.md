# Benchmark: finished v3 against transforms 2.4.0 on a Jetson AGX Orin

Measured on 2026-11-18. v3 is commit `93cc052` on `feature/v3`; the
baseline is master `ad5d2f7`, whose crate source, benches and lockfile are
identical to the `v2.4.0` tag. Both run the crate's own criterion suite
(`benches/benchmarks.rs`), default features. Raw criterion output of all
six runs: [`bench-v3-orin.txt`](bench-v3-orin.txt).

This is the measurement the "Internals" section of
[`DESIGN.md`](../DESIGN.md) did not have: its speed-up table comes from a
prototype without result labels, numerical changes, removal, re-parenting,
batch, cross-time or latest-common-time paths.

## Setup

- **Machine.** NVIDIA Jetson AGX Orin Developer Kit, aarch64, 12 cores,
  Ubuntu 24.04.4, glibc 2.39, `schedutil` governor, otherwise idle (load
  average 0.06 before the first run).
- **Binaries.** Cross-compiled on x86_64 with rustc 1.91.0 (stable),
  `cargo zigbuild` 0.23.4 with zig 0.17.1 as linker, target
  `aarch64-unknown-linux-gnu` against glibc 2.39, the repository's release
  profile (`lto = true`, `codegen-units = 1`). The same toolchain built
  both versions.
- **Runs.** Three rounds, alternating v3 and 2.4 within each round, every
  process pinned to one core with `taskset`. Criterion 0.7 with the
  suite's settings (3 s warm-up, 5 s measurement target, 1,000 samples in
  most groups).
- **Reading.** Times are criterion's point estimates. "2.4" and "v3" are
  the median of the three rounds; the speed-up is 2.4 time over v3 time,
  computed per round, given as its median and its range.

The rounds agree closely. The largest spread of one benchmark across its
three rounds is 10% (2.4, unknown frame on the 1,000-deep chain), then 4%
(2.4, `tree_climb_1k`) and 4% (2.4, `latest_common_time_4hop`); every
other benchmark stays within 3%.

## Results

| v3 benchmark | 2.4 benchmark | 2.4 | v3 | Speed-up | Range |
|---|---|---:|---:|---:|---:|
| `insert_prewarmed_1k` | `add_transform_prewarmed_1k` | 194.8 ns | 171.4 ns | 1.14x | 1.14-1.14x |
| `insert_prewarmed_60k` | `add_transform_prewarmed_60k` | 251.7 ns | 217.0 ns | 1.16x | 1.16-1.17x |
| `insert_prewarmed_100k` | `add_transform_prewarmed_100k` | 248.3 ns | 218.4 ns | 1.14x | 1.13-1.14x |
| `insert_all_8_frames_prewarmed_1k` | none | | 1.95 µs | | |
| `reparent` | none | | 101.9 ns | | |
| `lookup_1k` | `get_transform_1k` | 181.8 ns | 141.1 ns | 1.29x | 1.28-1.32x |
| `lookup_interpolated_1k` | `get_transform_interpolated_1k` | 284.5 ns | 247.6 ns | 1.15x | 1.14-1.15x |
| `robot_tree_map_to_camera_optical` | same | 707.4 ns | 650.6 ns | 1.09x | 1.08-1.12x |
| `robot_tree_leaf_to_leaf` | same | 1.11 µs | 343.8 ns | 3.23x | 3.22-3.25x |
| `robot_tree_lookup_across` | none | | 1.08 µs | | |
| `dynamic_chain_3hop_interpolated_1k` | same | 673.5 ns | 601.5 ns | 1.12x | 1.11-1.13x |
| `dynamic_chain_4hop_toward_ancestor` | same | 851.4 ns | 780.9 ns | 1.09x | 1.09-1.10x |
| `dynamic_chain_4hop_toward_descendant` | same | 912.0 ns | 785.5 ns | 1.16x | 1.16-1.17x |
| `siblings_below_16_dynamic_ancestors` | none | | 497.9 ns | | |
| `latest_common_time_4hop` | same | 326.0 ns | 67.2 ns | 4.84x | 4.76-5.02x |
| `latest_common_time_64hop` | none | | 897.1 ns | | |
| `lookup_across_1k` | `get_transform_at_1k` | 343.5 ns | 268.0 ns | 1.28x | 1.28-1.30x |
| `tree_climb_1k` | same | 68.83 µs | 37.34 µs | 1.84x | 1.77-1.89x |
| `tree_climb_1k_common_parent_elim` | same | 251.76 µs | 86.92 µs | 2.90x | 2.89-3.00x |
| `not_found_past_newest_1k` | same | 406.8 ns | 162.2 ns | 2.52x | 2.46-2.53x |
| `not_found_unknown_frame_1k` | same | 48.18 µs | 46.8 ns | 1,029x | 979-1,091x |

v3 is faster on every benchmark that exists in both suites.

## Against the prototype figures in DESIGN.md

| Workload | Prototype | Finished v3 |
|---|---:|---:|
| One hop, exact stamp, 1k samples | 1.51-2.19x | 1.29x |
| One hop, interpolated | 1.25-1.51x | 1.15x |
| Four dynamic hops, interpolated, either direction | 0.90-1.13x | 1.09x and 1.16x |
| Robot tree, leaf to leaf | 3.31-3.71x | 3.23x |
| Siblings below 16 dynamic ancestors | 24.10-28.46x | not measured against 2.4 |
| Static climb, 1,000 deep | 2.25-3.26x | 1.84x |
| Unknown frame, 1,000-deep chain | 1,027-1,270x | 1,029x |

- The topology-first gains carried over: leaf to leaf and the unknown
  frame are at the edge of or inside the prototype's range.
- The one-hop and static-climb gains are smaller than the prototype's.
  The finished lookup does what the prototype left out: it allocates the
  two names of its result, applies the rotation maintenance rule and
  checks finiteness. Which of these accounts for the difference was not
  profiled.
- The siblings workload has no benchmark in the 2.4 suite, so its
  prototype figure is neither confirmed nor contradicted. v3 takes 498 ns.

## Limits

- One machine, one architecture, one toolchain. The prototype figures
  were taken on a different machine, so the last table compares ratios
  across hardware.
- The baseline binary is cross-compiled like the v3 one; neither was
  built natively on the Orin.
- A run on an x86_64 laptop the same day was discarded: under the
  `powersave` governor and other load, two runs of the same 2.4 binary
  differed by a factor of 3 to 4.
- Benchmarks measure the workloads in `benches/benchmarks.rs` and nothing
  else. They set no latency budget for an application.
