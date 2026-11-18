Round 5: the maintainer ruled. analysis/v3-consensus/DESIGN.md (branch docs/v3-consensus-design, commit 067108c) now states the ruled design; read the "Rulings" section first, then the rest in full. Your task: find hidden issues the rulings cause. I am not asking whether you agree with them; they are made. I want consequences nobody stated: contradictions inside the document, contracts that became unimplementable or ambiguous, silent wrong-answer paths opened, loud failures users will hit in ordinary use, places where a ruling interacts badly with another ruling or with a 2.4 invariant that the document says stands.

The rulings that changed the text you last signed off on:
- M1: no Transform::transform_point. M2: no lookup_latest.
- M3: no Timestamp::from_secs_nanos / as_secs_nanos; a ROS-style conversion example goes in the Timestamp rustdoc instead (see the Time section; check that example).
- M15: no checked_add / checked_sub / checked_duration_since. Timestamp is from_nanos, as_nanos, MAX only.
- R1: Stamp::at is cut; Rotation::IDENTITY and Timestamp::MAX stay.
- R2: a self-lookup of a name the registry has never seen is UnknownFrame (you and I had kept 2.4's identity). I extended this to every name in every query, including `fixed` in lookup_across and both names in latest_common_time. Check that extension and the definition of "known" (has a slot: some stored edge names it as child or parent) against the slot lifecycle, remove_frame, reparent, and reclamation.
- R3: Transform::new and decoding accept target == source only with zero translation and exactly Rotation::IDENTITY, else a new TransformError::SelfTransformNotIdentity { frame }. I let compose-derived self-named results go unchecked (a ∘ a⁻¹ is identity only up to rounding). Check what that does to "every value is valid", to serde round trips, to inverse, to lookup results, and to CrossTimeTransform with equal endpoint names at different times.
- M10: new serde layout (fields target/source/stamp, ordered so that the frozen 2.4 postcard fixtures in tests/serde.rs fail to decode, pinned by a test). Check whether such an order actually exists for those fixtures, and whether the pin is meaningful.
- M13: empty names rejected at construction. M14: one expired member fails a whole batch. M16: LookupError::NonUnitRotation.
- M5 stands as a newtype Timestamp(i64).

Probe where you can (the 2.4 crate is in the repository; tests/serde.rs holds the frozen bytes). Keep the answer short: a ranked list, each item with the ruling(s) involved, the concrete scenario or contradiction, the evidence, and the smallest fix. Say what you did not check. Do not edit repository files.
