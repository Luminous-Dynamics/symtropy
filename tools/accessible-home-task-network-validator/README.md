# Accessible Home Task Network Validator

Standalone Rust semantic validator for synthetic resident task-network contracts v0.2 and v0.3.

## Run

From the repository root:

```sh
cargo fmt --manifest-path tools/accessible-home-task-network-validator/Cargo.toml
cargo fmt --manifest-path tools/accessible-home-task-network-validator/Cargo.toml -- --check
cargo test --manifest-path tools/accessible-home-task-network-validator/Cargo.toml

cargo run --manifest-path tools/accessible-home-task-network-validator/Cargo.toml -- \
  --input docs/built-environment/accessible-home-task-network-v0.2.example.json \
  --output /tmp/accessible-home-task-network-v02-report.json

cargo run --manifest-path tools/accessible-home-task-network-validator/Cargo.toml -- \
  --input docs/built-environment/accessible-home-task-network-v0.3.example.json \
  --output /tmp/accessible-home-task-network-v03-report.json
```

The CLI dispatches on `schema_version`. Malformed networks, unresolved references, broken paths, unsupported states, path expectation mismatches, or stale failure-mode expectations exit with code 2. If `--output` is omitted, the receipt is printed to stdout.

## v0.2 validation

- Strict typed JSON parsing, cross-kind unique IDs, edge endpoints, path-node/edge references, order and length.
- Each path ends at an outcome node; all declared nodes and edges participate in a path.
- Fallback/recovery edges require compatible non-normal triggers.
- Required task evidence is recomputed: a known barrier blocks; unknown/proposed/measurement/review-pending stays not-evaluable.
- Failure-mode goal-state expectations are recomputed from activated paths.

## v0.3 typed constraints

- Each resident constraint has a stable ID, kind (`hard` or `preference`), condition applicability, source reference, reviewer and note.
- Every path must supply exactly one finding per declared constraint.
- A condition-inapplicable constraint must be `not_applicable`; an applicable constraint must have a substantive disposition.
- A verified conflict with a hard constraint blocks that path; unresolved or insufficiently evidenced hard constraints make it not-evaluable.
- Preference conflicts and unresolved preferences appear as separate path counts and complete findings. They do not silently become hard blockers or disappear into a scalar score.
- The v0.3 receipt reports hard constraint counts separately from task evidence and preserves every constraint finding.

## Mutation coverage

The v0.2 suite checks the baseline, all six evidence states, barrier precedence, optional evidence, duplicate IDs, dangling references, path order/count, outcome termination, fallback triggers, evidence downgrades and deterministic receipts. The v0.3 suite additionally checks typed constraint coverage, duplicate/unknown/missing findings, condition applicability, hard conflict and unresolved semantics, unverified satisfaction, preference conflict visibility/non-gating, malformed state rejection, and deterministic receipts.

## Known limitations

In v0.3, constraint declarations and dispositions are still **claims** made by the synthetic fixture. The validator checks that declarations are consistent and evidence states are not overstated; it cannot prove that a resident actually stated a preference, that the evidence is truthful, or that a physical route/device behaves as described. `evidence_complete_for_fixture` applies only to the synthetic model, not to a real home. Real use requires consented resident co-design, actual site measurements, current local regulations, and qualified accessibility/building/life-safety review.

Do not store personal diagnoses or identifying resident data in fixtures. Use stable pseudonymous references and least-privilege access for any future real-project evidence store.
