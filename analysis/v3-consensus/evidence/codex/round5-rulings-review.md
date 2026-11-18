1. **R3 — derived self-transforms can be arbitrarily wrong, not merely rounded identities.**  
   Two accepted transforms, `a←b` translating +1 and `b←a` translating +1, compose to `a←a` translating +2. Its inverse translates −2. Thus [the rounding-only claim](analysis/v3-consensus/DESIGN.md:234) is false. Separately, my numerical probe of `a.compose(a.inverse())` produced a translation residual of ±4.44e−16 and failed decoding.

   **Smallest fix:** explicitly describe R3 as a constructor restriction, allowing arbitrary derived self-transforms and their inverses. Make serialization reject values that the decoder necessarily rejects, and restrict the [serde fixed-point requirement](analysis/v3-consensus/DESIGN.md:566) to encodable values. A universal self-identity invariant would require an additional decision about composition; silently replacing results with identity would conceal inconsistent inputs.

   Also explicitly exclude `CrossTimeTransform` from R3: equal endpoint names at different times legitimately produce motion. The baseline probe returned `camera←camera`, translation −2, across two timestamps. Building that result through `Transform::new` would now fail.

2. **M10 — a qualifying order exists, but the fixture pin can coexist with corrupted frame labels.**  
   I tested `translation, rotation, target, source, stamp`: it rejects all three frozen Transform fixtures and the Point fixture, while accepting its own valid roundtrip. However, a real 2.4 transform **`B←C @65 ns` decodes successfully as `A←B @−34 ns`**, with valid geometry and non-empty names.

   **Smallest fix:** retain the ruled fixture test, but characterize it as a regression pin, never a schema detector. Require positive v3 roundtrips too, and strengthen [the migration warning](analysis/v3-consensus/DESIGN.md:532): selecting the decoder requires external schema identification; mismatched decoding can corrupt **frames as well as time**.

3. **R2 × R3 — equal-time cross-time lookup remains ambiguous.**  
   With `camera` and `other` known but disconnected, consider:
   ```rust
   lookup_across("camera", t, "camera", t, "other")
   ```
   [“A known frame relative to itself at one instant is the identity”](analysis/v3-consensus/DESIGN.md:453) suggests success; resolving both fixed-frame legs requires `Disconnected`. Checking that every name exists does not settle this. The 2.4 probe returns `Disconnected`.

   **Smallest fix:** restrict the direct self-identity shortcut to `lookup` and `latest_common_time`. State that `lookup_across` validates both required legs even when endpoint names and times coincide; only endpoint-equals-fixed makes a leg empty.

4. **R2 — “known” must follow topology, not allocation history.**  
   Starting with the sole edge `p←c`, reparenting `c` under `q` makes **`p` unknown**, although nobody removed `p` explicitly. Removing the sole edge makes both endpoints unknown; draining its samples leaves both known. These now change self-query results. An isolated root cannot be registered through the current API because identity self-edges are rejected.

   [Reclamation “only when” unreferenced](analysis/v3-consensus/DESIGN.md:478) permits delayed memory reclamation, so “has a slot” is insufficient.

   **Smallest fix:** define known names exactly as endpoints of current topology edges, including drained edges. Remove their name-index visibility atomically when the last reference disappears, independently of memory reuse. Add these lifecycle cases to acceptance tests and document the isolated-root limitation.

5. **M15 — elapsed-time failure is not confined to integer limits.**  
   At ordinary timestamps, `(now - future) as u64` silently becomes a huge duration. The probe’s 1 s minus 2 s became **18,446,744,072.709551616 s**; `i64::checked_sub` alone returns `Some(-1_000_000_000)`. Removing `checked_duration_since` also removes its ordering check, not merely overflow protection.

   **Smallest fix:** add a rustdoc recipe alongside the ROS conversion:
   ```rust
   let elapsed = (now.as_nanos() >= earlier.as_nanos()).then(|| {
       Duration::from_nanos(now.as_nanos().abs_diff(earlier.as_nanos()))
   });
   ```
   This preserves the ruling without exporting another method.

The ROS conversion example passed boundary checks, including negative stamps, both `i32` extremes, invalid nanoseconds and outbound range rejection. [Scratch probe](~/.local/state/codex-review/20261117-105715-transforms.FeHVfX/scratch/round5/src/main.rs) used the real 2.4 serializer and a candidate validated v3 representation. I did not build integrated v3, exercise an implemented slot lifecycle, or run embedded/performance checks. Repository files were unchanged.