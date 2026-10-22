# Recovery Decision Corpus Auditor

A standalone Rust contract auditor for the synthetic cross-domain corpus at `docs/tech/recovery-decision-scenarios-v0.1.json`.

## What it proves

The auditor checks corpus-level invariants before a later reference oracle or planner can consume fixtures:

- valid schema/corpus identity and declared scenario count (17 frozen scenarios across 8 domain profiles);
- unique, non-empty scenario IDs and domain profiles;
- explicit `NO_ACTUATION_AUTHORIZED` claim ceiling;
- every fixture sets `expected_actuation=false`;
- each evidence record has a unique identifier, synthetic source reference, claim, method, evidence class, uncertainty kind, coverage and freshness classification;
- value-of-information examples have correct net-value arithmetic and a consistent inspect / do-not-inspect-on-value-alone label;
- mandatory gate statuses are limited to `PASS`, `BLOCK`, and `UNKNOWN`;
- `BLOCK` gates cannot be represented as generally eligible, and `UNKNOWN` gates cannot be represented as eligible;
- any preferred plan exists among the declared candidates and only appears on a human-review-eligible fixture;
- synthetic cost/yield values have bounded, non-negative forms;
- domain thresholds may not silently transfer between profiles;
- canonical input SHA-256 and the exact source commit are recorded in a machine-readable receipt.

This is **not** the decision oracle itself. It verifies the internal consistency of declared examples; it does not prove the estimates are accurate, the plans are optimal, or the domain gates are sufficient in real life.

## Run

From the repository root:

```sh
cargo test --manifest-path tools/recovery-decision-corpus-audit/Cargo.toml
cargo run --quiet --manifest-path tools/recovery-decision-corpus-audit/Cargo.toml -- \
  docs/tech/recovery-decision-scenarios-v0.1.json \
  --source-commit "$(git rev-parse HEAD)" > recovery-decision-audit-receipt.json
```

The executable writes a JSON receipt to stdout and exits non-zero on invalid input. The receipt includes the exact byte-level SHA-256 of the corpus and source commit, declared/unique scenario counts, domain count, gate/disposition counts, errors, and a claim ceiling. Do not compare receipts with different source commits or corpus digests as if they were the same evidence event.

## New hostile fixtures

- Performance screening can pass while an independent PV insulation/wet-leakage safety test remains unknown; the reuse operation stays held.
- One synthetic inspection has positive net information value after cost, delay and risk; a second has negative net value and does not justify that test *on value alone*. Both leave unresolved repair qualification held.
- The VOI values are invented arithmetic fixtures; the checker does not estimate real probabilities or certify that a test is worth doing in an actual operation.

## Evidence boundaries

- The bundled corpus is synthetic and tests rules/branches only.
- A passing audit is not a hosted CI pass unless the exact-head workflow run reports success.
- A passing audit is not domain qualification, legal permission, ecological clearance, engineering sign-off, or execution authorization.
- Rust compilation/test status must be taken from the actual run, not inferred from the existence of these files.
