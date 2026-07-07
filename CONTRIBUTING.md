# Contributing

Contributions are welcome. This document is short because the real rules live
in [AGENTS.md](AGENTS.md) — it is normative and covers style, the correctness
invariants, and the verification gate. Read it before writing code;
`tests/test_all.sh` runs the whole gate.

## Workflow

- Branch names: `bugfix/<topic>`, `feature/<topic>`, `docs/<topic>`
  (kebab-case).
- Commit messages: imperative summary line, then a body explaining *why* the
  change is needed — the history is documentation.
- Every correctness fix ships the regression test that fails on the old code
  in the same commit.
- Run the full gate (`tests/test_all.sh`) before opening a pull request.

## AI-assisted contributions

AI-assisted contributions are welcome, but they MUST follow AGENTS.md and
every AI-assisted commit MUST carry the trailer

```
Assisted-by: <Agent>:<model>
```

for example `Assisted-by: Claude:claude-fable-5`. See the
[AI-Assisted Development](README.md#ai-assisted-development) section of the
README for the rationale and details.

## Breaking changes

Breaking API changes (signatures, enum variants, trait bounds, public paths)
need maintainer sign-off before implementation. Open an issue describing the
change first; do not start with a pull request.

## When in doubt

Open an issue and ask. A loud question beats a silent guess.
