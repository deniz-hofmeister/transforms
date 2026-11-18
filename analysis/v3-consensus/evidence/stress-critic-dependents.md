# Downstream review of designer A's v3 draft

Reviewer role: downstream user. Draft: `claude-position.md` (draft 1).
Evidence: `analysis/dependents/{dimos,roslibrust_transforms,transforms_io}.md`,
`probes/dimos.rs`, `probes/roslibrust_transforms.rs` (first 90 lines),
`map-dependents.md`, `analysis/v3-review/REPORT.md` lines 770-800 (D6).

## 0. Method and limits

- I wrote a signature-only stub of the draft API
  (`evidence/critic-dependents/stub/src/lib.rs`,
  bodies `unimplemented!()`) and compiled the adapter sketches against it
  (`evidence/critic-dependents/sketch/src/main.rs`)
  with real `tokio 1.54.1`, `anyhow 1.0.104`, `thiserror 2.0.21`, `serde_json 1.0.154`.
- Commands (rustc 1.101.0-nightly 2026-10-18, x86_64 Linux, offline registry cache):
  `CARGO_TARGET_DIR=<scratch>/target cargo test --offline -- --nocapture` (2 tests pass, sketches compile, zero errors);
  `cargo run --offline --example serde_probe`.
- **Verified:** signatures, lifetimes, `Send`/`Sync`, error conversion. **Not verified:** any behaviour of the draft; nothing in the stub computes.
- The stub had to **assume** four things the draft does not state; each is a gap in the draft:
  1. `Edge<'a>` derives `Clone + Copy + Debug`. (Sketch S7b depends on `Copy`.)
  2. `BatchError` implements `Display + Error` (the draft's derive list is written over the enums; `BatchError` is a struct).
  3. The `Error` type of `TryFrom<SystemTime>`.
  4. `from_secs_nanos` returns `None` for `nanos >= 1e9`.
  Also unstated: the methods on `Stamp` (`at()`, `is_static()` exist in 2.4; dimos uses `.at()`), and which types get serde.
- Downstream line numbers are as cited by the reports. No dependent source was read or built. All dependents were studied against 2.1-2.3.
- Everything below about "what dimos/roslibrust/transforms_io would write" is my sketch, not their code.

## 1. Sketches (all compile against the stub)

Full text in the sketch file; the load-bearing parts:

```rust
// S1  TransformStamped -> Edge (borrowing the message)
fn edge<'m>(m: &'m TransformStamped, is_static: bool) -> Result<Edge<'m>, AdapterError> {
    let rotation  = Rotation::from_wxyz(r.w, r.x, r.y, r.z).map_err(g)?;          // fallible 1
    let transform = Transform::new([t.x, t.y, t.z].into(), rotation).map_err(g)?;  // fallible 2
    let stamp = if is_static { Stamp::Static } else {
        Timestamp::from_secs_nanos(sec.into(), nanosec).ok_or_else(..)?.into()     // fallible 3
    };
    Ok(Edge { parent: m.header.frame_id.trim_start_matches('/'),
              child:  m.child_frame_id.trim_start_matches('/'), transform, stamp })
}
// S2a bridges: per element, skip and log
for m in &msg.transforms { if let Err(e) = edge(m, st).and_then(|e| Ok(reg.insert(e)?)) { warn!("{e}") } }
// S2b dimos: atomic batch -- fallible conversion forces a collect
let edges = msg.transforms.iter().map(|m| edge(m, false)).collect::<Result<Vec<_>, _>>()?;
reg.insert_all(edges)?;
// S3  lookup -> wire: names and stamp threaded by hand
to_wire(target, source, at.into(), reg.lookup(target, source, at)?)
// S4  latest
let stamp = reg.latest_common_time(target, source)?;
let at = match stamp { Stamp::At(t) => t, Stamp::Static => Timestamp::from_nanos(0) };
to_wire(target, source, stamp, reg.lookup(target, source, at)?)
// S5  wait loop
fn classify(e: &LookupError, max_age: Duration) -> Verdict { match e {
    UnknownFrame{..} | Disconnected{..} | NoData{..} => Retry,
    OutOfRange{requested, latest, ..} if requested > latest => Retry,
    OutOfRange{requested, latest, ..} => match latest.checked_sub(max_age) {
        Some(floor) if *requested >= floor => Retry, _ => Permanent },
    _ => Permanent } }
// loop: arm Notify; lookup under read guard; Permanent -> Err(e); timeout -> Timeout(last_err)
// S6  flatten
let transform = reg.lookup(parent, child, at)?;
reg.insert(Edge { parent, child: as_child, transform, stamp: at.into() })?;
// S7  clock reset
reg.remove_samples_through(Timestamp::MAX);                         // (a) bridge saw the clock step back
match reg.insert(e) {                                               // (b) REPORT D6 by-cause recovery
    Err(InsertError::Expired { frame, newest, .. }) if newest > now => { reg.remove_frame(&frame); reg.insert(e)? }
    r => r? }
```

## 2. Signature findings (compiled)

| Question | Result |
|---|---|
| `insert_all` with an iterator borrowing `&TFMessage` | Works (`S2c`). With a **fallible** per-message conversion (every real adapter: rotation, finite, time) the iterator cannot be passed directly; one `Vec<Edge>` per message is needed (`S2b`). 2.4 `add_transforms` had the same shape, with `String`s. `BatchError.index` then indexes the collected `Vec`, which equals the message index only if nothing was filtered first. |
| `insert_all` over `msg.transforms.into_iter()` (consuming owned messages) | Impossible by construction: an `Edge` cannot borrow from an item the iterator has moved. Adapters must iterate by reference. Not a real loss; no studied adapter needs to consume. |
| Computed names (`tf_prefix`, `format!("{ns}/{name}")`) | Needs a side `Vec<(String, String)>` that outlives the edges (`S2d`). Clumsier than 2.4, where the owned `Transform` took the `String`. Stripping a leading `/` is a borrow and costs nothing. No studied adapter prefixes (unverified for dimos). |
| `Edge` across `.await` / in spawned tasks | Fine. `Edge<'_>: Send + Sync`; the ingest future (owned message off an `mpsc`, write guard, edges built after the guard) is `Send`. No edge lives across an await in any natural shape. |
| `tokio::sync::RwLock` / `std::sync::RwLock` guards | Better than 2.4: the result is `Copy` and borrows nothing; errors own their `String`s. Nothing ties a result to the guard. |
| Actor pattern (edges sent over a channel to the registry's owner) or any queue of pending edges | `Edge<'a>` cannot cross. The user defines an `OwnedEdge` and converts (`actor` in the sketch). 2.4's `Transform<T>` was that owned, cloneable, serde-able record. No studied adapter does this (all hold the lock and the ROS message), so evidence of need is nil; the gap is real for non-adapter users. |
| `anyhow` `?` | Works for all four error types (given assumption 2). |
| `thiserror` `#[from]` / `#[error(transparent)]` | Works. `LookupError` is not generic, so roslibrust's inferred obstacle F5 is gone. Because errors are `Clone`, `Timeout(LookupError)` can carry the last cause; 2.4's `RegistryError` derives only `Error, Debug` (`src/core/registry/error.rs:22`), so this is new. |
| `Send + Sync + 'static` | `Registry`, `Transform`, `Edge<'static>`, all errors: yes (static assertions compile). |
| serde on `Edge<'a>` (if the draft intends it) | A trap. Run: `serde_json::from_str::<Edge>("{\"parent\":\"ma\\u0070\",...}")` -> `Err("invalid type: string \"map\", expected a borrowed string at line 1 column 20")`; `from_reader::<_, Edge>` does not compile (`higher-ranked lifetime error`). A borrowed-`&str` record deserializes only from an in-memory buffer with no escapes. The draft keeps "optional serde" without saying on which types; 2.4's frozen wire format was the labelled `Transform<T>`, which no longer exists. Zero studied users enable serde. |

## 3. Where the draft is more awkward than 2.4

1. **Three fallible steps per message instead of one** (`Rotation::from_wxyz`, `Transform::new`, time), two error types. Cosmetic; one `map_err` closure.
2. **Results carry no names and no stamp.** roslibrust's `Transform -> TransformStamped` conversions become four-argument functions. Both bridges already hold the arguments, so the cost is small. The real loss is transparency: `Debug` of a result no longer shows direction. The transforms_io probe output `Ok(-0.3, laser<-base_link)` (hazard #8) is how a backwards call is recognized today; with the draft the same call prints bare numbers.
3. **Timestamp -> wire has no inverse of `from_secs_nanos`, and the signed type makes the obvious code wrong.** roslibrust's `as_ros_time` body (`nanos / 1e9`, `nanos % 1e9`, `as u32`), correct on `u64`, applied to the draft's `i64`: test `neg_split` prints `n=-500000000 naive=(0, 3794967296) right=(-1, 500000000)`. The draft cures the inbound clamp (need #6) and opens an outbound sign bug of the same family as the `u128` sign-extension the 2.0 migration fixed. An `as_secs_nanos(self) -> (i64, u32)` closes it; every adapter publishes, so the need is demonstrated.
4. **`Timestamp -> SystemTime` is missing** (only `TryFrom<SystemTime> for Timestamp`). See section 5.
5. **No owned edge value** (section 2).
6. **`Stamp::Static => <instant>` arm at every latest call site survives** (F2, need #4), and the result no longer carries a stamp, so the caller must keep the `Stamp` beside the transform.
7. **No `Timestamp::now()`.** A staleness check (`now - latest`) becomes `Timestamp::try_from(SystemTime::now())?`. Side effect, positive: transforms_io's wrong `lookup(.., now())` latest pattern gets harder to write than the right one.
8. **`insert_all` + `Expired` changes dimos's batch semantics**: one late member now rejects the whole TF message where 2.4 returned `Ok` and evicted. Loud and arguably right, but it is a behaviour change on dimos's only insert path; a mixed-publisher message with one straggler is dropped whole.
9. **No `Default`/`new` on `Registry`**: `#[derive(Default)]` on an adapter struct holding one breaks. Trivial; zero users call `new`.
10. **Input bits are not preserved**: `Rotation` is "stored normalized", so a sample looked up at its own stamp no longer returns the submitted `f64`s (the roslibrust report lists "exact f64 round trip" as a property). The draft's "single-hop stays bit-exact" is exact relative to the stored, normalized value only. No studied user depends on it (transforms_io's shadow log stores messages).

Easier than 2.4: no name clones on the hot path (F1), no `String` allocation per insert, no E0283, total `inverse`/`Mul`, non-generic cloneable errors, `Edge: Copy` removes REPORT's "rebuild is a visible cost of item 10" (S7b re-inserts the same value), flatten is two lines with no fallible rebuild, the `Stamp` from `latest_common_time` drops straight into an `Edge`.

## 4. Ranked user needs (map-dependents.md section 7)

| # | Need | Draft | Note |
|---|---|---|---|
| 1 | Diagnosable insert rejections | **Met** | Every structural variant names frames; `BatchError.index`. `GeometryError` names nothing, but the caller holds the message. |
| 2 | Surviving a clock reset | **Partly met** | `Expired` is loud; `remove_samples_through(Timestamp::MAX)` removes the hole and the far-future recipe. A step smaller than `max_age` still interleaves two epochs silently (REPORT D6 says so). The clear is registry-wide; per-frame is `remove_frame` + re-insert, which releases the pin. Unstated in the draft: that the clear also resets each frame's expiry anchor (map-internals: expiry is relative to the buffer's newest key, so presumably yes). |
| 3 | Gap / staleness bound | **Missed** | Candidate 2, out. Still not buildable outside. |
| 4 | "Latest" | **Unchanged** | Static arm stays; two calls under one guard stay; private-lock adapters still cannot offer it atomically. Slightly worse: no stamp on the result. |
| 5 | Permanent vs transient | **Improved, not met** | Same information as 2.4's `NotFoundAt`, better shape (concrete time, `Clone`, `NoData`/`OutOfRange` split). The classification is still user code (S5, 10 lines) and needs the adapter's own `max_age`. `UnknownFrame`/`Disconnected` are transient at startup and permanent later; the type cannot say which. |
| 6 | Middleware time in | **Met inbound, worsened outbound** | Section 3 item 3. |
| 7 | Frame names | **Unchanged** | `EmptyFrameName` at insert, as 2.4. T1 (hand-composition joining on `""`) disappears with labelled `Mul`. |
| 8 | Looser rotations | **Missed, slightly worse** | Same 1e-6 rejection; `Quaternion::normalize` is gone, so the outside fix is hand arithmetic instead of one call. |
| 9 | Finite lookup outputs | **Met** | If the exit check or the translation bound ships. dimos's `validate()` on every result becomes dead code, and its Hazard 5 (rejecting a drifted long chain) disappears with the unit `Rotation`. |
| 10 | Kind change / re-parent as a decision | **Partly met** | Names in errors. Kind change is still `remove_frame` + insert. No read-out. |
| 11 | E0283 | **Met** | |
| 12 | Lookup direction | **Slightly worsened** | Still two positional `&str`; the labelled result that exposed a swap in `Debug` is gone. |

## 5. dimos without `SystemTime` as the registry time type

- dimos's wire unit is signed ROS nanoseconds, converted to `SystemTime` only to feed `Registry<SystemTime>` (`tf.rs:50-81`). Against the draft that conversion **disappears**: `Timestamp::from_nanos(ns)`.
- Gains: no `covers [NaN, NaN]` on pre-epoch data (#12; pre-epoch is a tested dimos path); `Timestamp::MAX` replaces the far-future recipe; static-on-the-wire is `from_nanos(0)` instead of `UNIX_EPOCH`.
- Losses:
  - Range: `i64` ns is 1677-2262; `SystemTime` is wider. `TryFrom` is fallible where the 2.4 path was not.
  - If dimos's Rust API takes or returns `SystemTime` (**not established by the reports**), it needs both directions. The draft ships one. The reverse is 4 lines with a sign branch (`to_system` in the sketch); `Duration::from_nanos(n as u64)` on a negative stamp is the wrong version waiting to be written.
- roslibrust loses its generic `TransformManager<T>`, the `RosTimestamp` trait for two types and `MockTimestamp`. That is deletion on their side, except for any of its users who picked `SystemTime` (unknown, 87 downloads).

## 6. What users must now build, by the AGENTS.md scope test

| Substitute | Exact on the draft's public API? | Verbose or wrong-answer generator |
|---|---|---|
| Labelled/stamped result (names + stamp beside the `Transform`) | Yes | Verbose. One footgun: flatten/re-publish types the names twice; `Edge { parent: source, child: target }` silently stores the inverse. 2.4 had the same risk only when renaming. |
| Owned edge record (queues, channels, logs) | Yes | Verbose. |
| Timestamp -> `(sec, nsec)` | Yes | **Wrong-answer generator** (measured above; silent on negative stamps). |
| Timestamp -> `SystemTime` | Yes | Mild generator (sign branch). |
| Wait-for-transform + classification | Yes | Verbose. A misclassification times out or fails early; both loud. |
| "Latest" | Yes, under one guard | Generator by the record: 1 of 3 wrong for seven releases, a second teaches `now()` in its examples. With `now()` gone the wrong version is less tempting, not impossible. |
| Normalize a loose quaternion | Yes | Verbose; a zero quaternion yields NaN, which `from_wxyz` rejects loudly. Unbounded normalization is policy. |
| Clock-reset detection | Bridge clock: yes. Per edge from stamps: yes, with a shadow `HashMap<child, Timestamp>` the adapter fills itself (it sees every insert). | Policy-heavy. Clearing on every `Expired` lets one straggler (a publisher stamping 0) wipe history on each message; REPORT D6 already warns. Availability bug, not a wrong pose. |
| Gap bound | **No** | Core or nowhere. |
| Typed frames | Yes | Verbose. |

## 7. The six "unsure" candidates, on downstream evidence only

1. **`lookup_latest`.** For. Every ROS-facing user must implement time-0 semantics (3 of 3 build it); one got it wrong for seven releases after `latest_common_time` shipped (a discoverability failure a method next to `lookup` fixes); both bridges hide the lock so their users cannot do the two-call form atomically; `(Stamp, Transform)` is exactly what dimos emits (`Static` -> wire 0) and removes the arbitrary-instant arm. Against: exact outside, so the scope rule's first bullet says no, and the dependents study ruled it out. Consistency point: the draft already admits `from_secs_nanos` on identical grounds (exact outside, 2 of 4 wrong). Admit both or neither.
2. **Maximum interpolation gap.** The hazard is the best-evidenced silent one and is core-or-nowhere; the *shape* has one downstream precedent only: dimos's old per-query tolerance (nearest sample within a tolerance, default the whole window, i.e. effectively unbounded). Nobody asked; no user stated a value. Evidence supports the draft's "out of 3.0", and does not support any of the three shapes over the others.
3. **Normalizing rotation constructor.** Weak. Loud failure, frequency unmeasured, two adapters affected, neither reported it. Exact outside. But the draft makes the outside version worse by deleting `normalize`; if it stays out, the rustdoc of `from_wxyz` owes the five-line recipe and the tf2 tolerance difference (rec 12, still undocumented).
4. **`frames()`.** Against. The one cited user (transforms_io shadow log) keeps entries the registry *refused*; the report calls it "a message-log need, not a registry-state need", so a read-out would not replace it. whirl_tf's `remove_frame` loop is replaced by `remove_samples_through(MAX)`. No studied user enumerates frames.
5. **Drop `std`.** All studied users are std; the feature costs them nothing. It buys one impl used by one user (dimos), who no longer needs it if it feeds `i64` ns directly. The hand-rolled replacement is sign-sensitive `u128 -> i64` code, the class the adapters already got wrong once. Either complete it (both directions) or drop it; shipping one direction is the worst of the three.
6. **Lookup argument struct.** Moderately for, and stronger than the evidence behind `Edge`'s named fields: the only observed direction bug in any dependent is a lookup swap (transforms_io, 3 of 4 backends, 12 releases); no dependent has ever swapped parent/child on insert (they copy `header.frame_id`/`child_frame_id` mechanically). The draft applies the named-field argument where there is no bug and withholds it where there is one, while also removing the labelled result that made the swap visible. Cost: verbosity on the most frequent call and the loss of tf2's positional order that pelorus relies on.

## 8. Other defects in the draft seen from downstream

- `Edge` with `pub` fields and literal construction cannot be `#[non_exhaustive]`; any later field is a major break.
- `NoCommonTime { frame, .. }` is not a specification; S5 cannot classify it.
- `KindMismatch { pinned_static: bool }`: a bool where the crate already has the static/dynamic vocabulary; fine for logs.
- `reparent(seed: Edge)` reparents `seed.child`; nothing at the call site says so.
- Error `String` payloads allocate on every failing call. A wait loop or a persistent parent conflict at 100 Hz allocates per message. Not measured; likely irrelevant.
- The draft's "No studied user consumes the labels through a checked path" is true; it omits that roslibrust reads them through accessors on the way out and that the labels are the only direction diagnostic in `Debug`.
