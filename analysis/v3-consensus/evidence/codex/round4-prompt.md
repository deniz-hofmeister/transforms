Round 4: confirmation. I rewrote analysis/v3-consensus/DESIGN.md with your round-3 edits. Please re-read it in full and confirm or correct. Keep the answer short: a verdict line, then only what is still wrong, with exact replacement text.

What I did with your edits:
- Adopted: every factual correction in your section 3 (count 79, 14 variants, five types and two traits, the band/Newton/inverse/ratio/exactness wording, time claims, internals bullets, differential paragraph, adopted-variant timing table, serde hazard wording, review wording, gap-limit row, split exclusions, limits); LookupError::NonUnitRotation { norm } (ruling j); all section 4 contracts (maintenance rule with 1.0/libm::sqrt, inverse operation order, rotate is raw, composition and lookup rules, cross-time rules, NoCommonTime.latest_possible, diagnosis not cut short by a numerical failure, Expired boundary rules, no hidden watermark, checked arithmetic over the full range).
- One place I did not follow you, and recorded as a split instead (M10): the serde layout. You recommend keeping 2.4's field names/order and variant indices with an i64 stamp. A checker probed that exact layout: a real 2.4.0 postcard Transform decodes Ok into it with a wrong instant (1.7e18 ns -> 8.5e17). My preference is to rename the fields to target/source/stamp and choose the order so the frozen 2.4 postcard fixtures in tests/serde.rs fail to decode, pinned by a test: old JSON fails loudly, old postcard fails for those fixtures (not provably for all bytes). I kept the `Stamp { Static, At }` declaration order as in 2.4 in the API block either way. Check that M10 states your position fairly, and say if the probe changes your view.
- Added from four independent checkers (evidence/verify-findings.md), please check these for errors:
  M13: empty names rejected at Transform::new and decoding, lookup("", "", t) is UnknownFrame; this changes 2.4 ruling D6.
  M14: one expired member fails the whole batch (2.4 stored the other frames' fresh samples); declared, with the fallback in rustdoc and the ingest example.
  M15: checked_add / checked_sub / checked_duration_since are exact outside on as_nanos, so they are a scope exception; checked_add is first to cut.
  M16: LookupError::NonUnitRotation as a variant for a failure believed unreachable.
  M8 now records the positional from_wxyz regression (xyzw identity accepted as a half turn) and "Not in 3.0" has a named-field rotation input row.
  Residuals recorded: sub-max_age clock jumps and any jump on unbounded(); lookup_across with swapped times or a moving frame as `fixed`; self-lookup of an unregistered name is identity; a self-named non-identity transform is constructible.
  remove_frame return value on roots; slot reclamation never renumbers; slot lifecycle unverified (Limits and Acceptance).
  Derive list completed; "only functions marked const are promised const".
- The authorship line currently says: "In round 3 Codex signed off subject to corrections; this document incorporates them". If you confirm, I will add that round 4 confirmed the text.

Do not edit repository files.
