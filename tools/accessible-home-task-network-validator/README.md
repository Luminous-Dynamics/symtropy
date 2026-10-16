# Accessible Home Task Network Validator

Standalone Rust semantic validator for the **synthetic** v0.2 resident task-network contract.

## Run

From the repository root:

```sh
cargo fmt --manifest-path tools/accessible-home-task-network-validator/Cargo.toml
cargo fmt --manifest-path tools/accessible-home-task-network-validator/Cargo.toml -- --check
cargo test --manifest-path tools/accessible-home-task-network-validator/Cargo.toml
cargo run --manifest-path tools/accessible-home-task-network-validator/Cargo.toml -- \
  --input docs/built-environment/accessible-home-task-network-v0.2.example.json \
  --output /tmp/accessible-home-task-network-report.json
```

Malformed networks, unresolved references, broken paths, unsupported states, path expectation mismatches, or stale failure-mode expectations exit with code 2. If `--output` is omitted, the receipt is printed to stdout.

## What is checked

- Strict typed JSON parsing rejects unsupported enum values and unknown object fields.
- IDs are unique across goal, node, edge, path, failure-mode and network entities.
- Edge endpoints, path node/edge references, edge ordering and path lengths resolve exactly.
- Each path ends at an outcome node; every declared node and edge participates in a path.
- Fallback/recovery edges require a non-normal trigger compatible with their path.
- Failure triggers agree with activated path conditions; affected nodes belong to a non-activated primary path.
- Path assessments are recomputed from evidence. Required barriers dominate; all required evidence gaps stay not-evaluable; optional node evidence is advisory. Edges in a path are considered required until an explicit advisory-edge field is added in a future schema.
- Failure-mode expectations are recomputed from triggered path states.
- The receipt includes SHA-256 of the exact input bytes and repeats the synthetic claim boundary.

## Mutation corpus

Unit tests cover the valid baseline, all six evidence states, required-barrier precedence, optional evidence, duplicate cross-kind IDs, dangling endpoints/references, broken edge order/count, invalid goal endpoint, fallback trigger, required-evidence downgrade, stale goal-state claims, invalid enums, real-home fixture rejection, empty failure activation and deterministic receipts.

## Known contract limitation

In v0.2, resident constraints are prose strings rather than typed, path-specific dispositions. The validator deliberately does **not** infer that a path satisfies those constraints. A follow-on schema revision should give each constraint a stable ID and each path an explicit disposition/evidence record (satisfied for fixture, conflicted, or unresolved), then make unresolved hard constraints non-passing. Until then, `evidence_complete_for_fixture` means only that the nodes and edges on that synthetic path carry the declared complete evidence state.

## Important boundary

This validates consistency of the *declared synthetic graph*. It does not prove that an evidence claim is true; it does not simulate bodies, geometry, assistive technology, fire/egress, privacy, security, or real-world reliability; and it does not qualify or certify a real home. Human co-design, actual site measurement, local jurisdiction checks and qualified professional review remain separate.
